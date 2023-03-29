//! Interfaces to EMV payment cards.
//!
//! The EMV specifications can be downloaded freely from EMVCo's website, and
//! also contain a recap of ISO 7816.
//!
//! Except where otherwise noted, data elements are defined in EMV Book 3, Annex A.
//! Some tags are proprietary to specific payment systems, in which case their sources
//! are either linked or referred to by shorthand:
//! - [neaPay]: https://neapay.com/online-tools/emv-tags-list.html

use crate::{ber, iso7816, util, Result};
use pcsc::Card;
use tap::TapFallible;
use tracing::{trace_span, warn};

pub const DIRECTORY_DF_NAME: &str = "1PAY.SYS.DDF01";

/// The EMV Directory, also known as the Payment System Environment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Directory {
    /// 0x88: SFI of the Directory Elementary File. (Values 1-30.)
    pub ef_sfi: u8,

    /// 0x5F2D: Language Preference. (an2, 2-8)
    /// List of 2-character language codes, eg. "enfr" (English, French).
    pub lang_prefs: Option<String>,

    /// 0x9F11: Issuer Code Table Index. (n2, 1)
    /// ISO/IEC 8859 code table for displaying the Application Preferred Name.
    pub issuer_code_table_idx: Option<u8>,

    /// 0xBF0C: FCI Issuer Discretionary Data. (var, <=222)
    pub fci_issuer_discretionary_data: Option<FCIIssuerDiscretionaryData>,
}

impl<'a> Directory {
    pub fn select(card: &mut Card, wbuf: &mut [u8], rbuf: &'a mut [u8]) -> Result<Self> {
        iso7816::select_name(card, wbuf, rbuf, DIRECTORY_DF_NAME.as_bytes())
    }
}

impl<'a> TryFrom<&'a [u8]> for Directory {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self> {
        let span = trace_span!("Directory");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x88] => slf.ef_sfi = *value.first().unwrap_or(&0),
                &[0x5F, 0x2D] => slf.lang_prefs = Some(String::from_utf8_lossy(value).into()),
                &[0x9F, 0x11] => slf.issuer_code_table_idx = Some(*value.first().unwrap_or(&0)),
                &[0xBF, 0x0C] => {
                    slf.fci_issuer_discretionary_data = match value.try_into() {
                        Ok(v) => Some(v),
                        Err(err) => {
                            warn!("couldn't parse 0xBF0C: {:}", err);
                            None
                        }
                    }
                }
                _ => warn!("unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

/// 0xBF0C: FCI Issuer Discretionary Data. (var, <=222)
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FCIIssuerDiscretionaryData {
    /// 0x9F4D: Log Entry (SFI and number of records). (b, 2)
    pub log_entry: Option<(u8, u8)>,
    //// 0x9F5D: [Mastercard] Application Capabilities Info (ACI). (b, 3) [neaPay]
    pub app_capability_info: Option<(u8, u8, u8)>,
    /// 0x9F0A: Application Selection Registered Proprietary Data. (b, var)
    /// Simple TLV format: u16 tag, u8 length, [length] data.
    pub app_selection_reg_propr_data: Option<Vec<(u16, Vec<u8>)>>,
    /// 0x9F5E: Data Storage Identifier. (n16-22, 8-11) [neaPay]
    /// The PAN (card number) as hex digits, then the sequence number if applicable, eg.
    /// "5355 2205 1234 5678" -> [ 0x53, 0x55, 0x22, 0x05, 0x12, 0x34, 0x56, 0x78 ].
    pub ds_id: Option<Vec<u8>>,
    /// 0x9F6E: ???
    pub unknown_9f6e: Option<Vec<u8>>,
}

impl<'a> TryFrom<&'a [u8]> for FCIIssuerDiscretionaryData {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self> {
        let span = trace_span!("FCIIssuerDiscretionaryData");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x9F, 0x4D] => {
                    slf.log_entry =
                        Some((*value.first().unwrap_or(&0), *value.last().unwrap_or(&0)))
                }
                // There are two known tags with this ID, according to [neaPay]:
                // - [Mastercard] Application Capability Info, length 3.
                // - [???] Available Offline Spending Amount, length 6.
                &[0x9F, 0x5D] if value.len() == 3 => {
                    slf.app_capability_info = Some((value[0], value[1], value[2]))
                }
                &[0x9F, 0x0A] => {
                    let mut data = value;
                    let mut tvs = vec![];
                    while data.len() > 0 {
                        let (tb, rest) = data.split_at(2);
                        let (len, rest) = rest.split_first().unwrap();
                        let (value, rest) = rest.split_at(*len as usize);
                        data = rest;
                        tvs.push((u16::from_be_bytes([tb[0], tb[1]]), value.into()));
                    }
                    slf.app_selection_reg_propr_data = Some(tvs);
                }
                &[0x9F, 0x5E] => slf.ds_id = Some(value.into()),
                &[0x9F, 0x6E] => slf.unknown_9f6e = Some(value.into()),
                _ => warn!("unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryRecord {
    /// 0x60: A single entry.
    pub entry: DirectoryRecordEntry,
}

impl TryFrom<&[u8]> for DirectoryRecord {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let span = trace_span!("DirectoryRecord");
        let _enter = span.enter();

        let (_, (tag, value)) = ber::parse_next(data)?;
        util::expect_tag(&[0x70], tag)?;

        Ok(Self {
            entry: value.try_into()?,
        })
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryRecordEntry {
    /// 0x61: List of application definitions.
    pub applications: Vec<DirectoryApplication>,
}

impl TryFrom<&[u8]> for DirectoryRecordEntry {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self> {
        let span = trace_span!("DirectoryRecordEntry");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x61] => slf.applications.push(value.try_into()?),
                _ => warn!("unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryApplication {
    /// 0x4F: SELECT'able ADF name.
    pub adf_name: Vec<u8>,
    /// 0x50: Human-readable label.
    pub app_label: String,
    /// 0x9F12: Human-readable preferred (display) name.
    pub app_preferred_name: Option<String>,
    /// 0x87: DirectoryApplication Priority Indicator. (TODO: Parse.)
    pub app_priority: Option<u8>,
    /// 0x73: Directory Discretionary Template.
    pub dir_discretionary_template: Option<Vec<u8>>,
}

impl TryFrom<&[u8]> for DirectoryApplication {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self> {
        let span = trace_span!("DirectoryApplication");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x4F] => slf.adf_name = value.into(),
                &[0x50] => slf.app_label = String::from_utf8_lossy(value).into(),
                &[0x9F, 0x12] => {
                    // Technically incorrect; this isn't UTF-8, but the charset in Directory.
                    slf.app_preferred_name = Some(String::from_utf8_lossy(value).into())
                }
                &[0x87] => slf.app_priority = value.get(0).copied(),
                &[0x73] => slf.dir_discretionary_template = Some(value.into()),
                _ => warn!("unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Application {
    /// 0x50: Human-readable label, in ASCII(ish).
    pub app_label: String,
    /// 0x87: DirectoryApplication Priority Indicator. (TODO: Parse.)
    pub app_priority: Option<u8>,
    /// 0x9F38: Processing Options Data Object List (PDOL).
    /// A list of data elements the card wants in a GET PROCESSING OPTIONS.
    pub pdol: Option<Vec<(u32, usize)>>,
    /// 0x5F2D: Language Preference. (an2, 2-8)
    /// List of 2-character language codes, eg. "enfr" (English, French).
    pub lang_prefs: Option<String>,
    /// 0x9F11: Issuer Code Table Index. (n2, 1)
    /// ISO/IEC 8859 code table for displaying the Application Preferred Name.
    pub issuer_code_table_idx: Option<u8>,
    /// 0x9F12: Human-readable preferred (display) name, in indicated charset.
    pub app_preferred_name: Option<String>,
    /// 0xBF0C: FCI Issuer Discretionary Data. (var, <=222)
    pub fci_issuer_discretionary_data: Option<FCIIssuerDiscretionaryData>,
}

impl Application {
    pub fn select<'a>(
        card: &mut Card,
        wbuf: &mut [u8],
        rbuf: &'a mut [u8],
        name: &[u8],
    ) -> Result<Self> {
        iso7816::select_name(card, wbuf, rbuf, name)
    }
}

impl TryFrom<&[u8]> for Application {
    type Error = crate::Error;

    fn try_from(data: &[u8]) -> Result<Self> {
        let span = trace_span!("Application");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x50] => slf.app_label = String::from_utf8_lossy(value).into(),
                &[0x87] => slf.app_priority = value.get(0).copied(),
                &[0x9F, 0x38] => {
                    slf.pdol = parse_pdol(value)
                        .tap_err(|err| warn!("Couldn't parse <0x9F38> PDOL: {}", err))
                        .ok()
                }
                &[0x5F, 0x2D] => slf.lang_prefs = Some(String::from_utf8_lossy(value).into()),
                &[0x9F, 0x11] => slf.issuer_code_table_idx = value.first().copied(),
                &[0x9F, 0x12] => {
                    // Technically incorrect; this isn't UTF-8, but the charset in Directory.
                    slf.app_preferred_name = Some(String::from_utf8_lossy(value).into())
                }
                &[0xBF, 0x0C] => {
                    slf.fci_issuer_discretionary_data = value
                        .try_into()
                        .tap_err(|err| {
                            warn!(
                                "couldn't parse <0xBF0C> FCI Issuer Discretionary Data: {:}",
                                err
                            )
                        })
                        .ok()
                }
                _ => warn!("unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

fn parse_pdol(mut data: &[u8]) -> Result<Vec<(u32, usize)>> {
    let mut pdol = vec![];
    while data.len() > 0 {
        let (rest, tag) = ber::take_tag(data).map(|(i, v)| (i, ber::tag_to_u32(v)))?;
        let (rest, len) = ber::take_len(rest)?;
        data = rest;
        pdol.push((tag, len));
    }
    Ok(pdol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directory_selection() {
        // `SELECT '1PAY.SYS.DDF01'` response from an old Curve card.
        let rsp: iso7816::SelectResponse = [
            0x6F, 0x26, 0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44,
            0x44, 0x46, 0x30, 0x31, 0xA5, 0x14, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E,
            0x9F, 0x11, 0x01, 0x01, 0xBF, 0x0C, 0x05, 0x9F, 0x4D, 0x02, 0x0B, 0x0A,
        ][..]
            .try_into()
            .expect("couldn't parse SelectResponse");
        assert_eq!(rsp.fci.df_name, "1PAY.SYS.DDF01".as_bytes());
        assert_eq!(
            rsp.fci.pt,
            Some(
                &[
                    0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01, 0xBF,
                    0x0C, 0x05, 0x9F, 0x4D, 0x02, 0x0B, 0x0A,
                ][..]
            )
        );

        let dir: Directory = rsp
            .parse_into()
            .expect("couldn't parse SelectResponse into Directory");
        assert_eq!(
            dir,
            Directory {
                ef_sfi: 1,
                lang_prefs: Some("en".into()),
                issuer_code_table_idx: Some(1),
                fci_issuer_discretionary_data: Some(FCIIssuerDiscretionaryData {
                    log_entry: Some((11, 10)),
                    ..Default::default()
                }),
            }
        );
    }

    #[test]
    fn test_parse_directory_record() {
        let rsp: iso7816::ReadRecordResponse = [
            0x70, 0x40, 0x61, 0x3E, 0x4F, 0x07, 0xA0, 0x00, 0x00, 0x00, 0x04, 0x10, 0x10, 0x50,
            0x10, 0x44, 0x65, 0x62, 0x69, 0x74, 0x20, 0x4D, 0x61, 0x73, 0x74, 0x65, 0x72, 0x63,
            0x61, 0x72, 0x64, 0x9F, 0x12, 0x10, 0x44, 0x65, 0x62, 0x69, 0x74, 0x20, 0x4D, 0x61,
            0x73, 0x74, 0x65, 0x72, 0x63, 0x61, 0x72, 0x64, 0x87, 0x01, 0x01, 0x73, 0x0B, 0x9F,
            0x0A, 0x08, 0x00, 0x01, 0x05, 0x01, 0x00, 0x00, 0x00, 0x00,
        ][..]
            .into();
        let rec: DirectoryRecord = rsp
            .parse_into()
            .expect("couldn't parse ReadRecordResponse into DirectoryRecord");
        println!("{:#02X?}", rec);
        assert_eq!(
            rec,
            DirectoryRecord {
                entry: DirectoryRecordEntry {
                    applications: vec![DirectoryApplication {
                        adf_name: vec![0xA0, 0x0, 0x0, 0x0, 0x4, 0x10, 0x10],
                        app_label: "Debit Mastercard".into(),
                        app_preferred_name: Some("Debit Mastercard".into()),
                        app_priority: Some(1),
                        dir_discretionary_template: Some(vec![
                            0x9F, 0xA, 0x8, 0x0, 0x1, 0x5, 0x1, 0x0, 0x0, 0x0, 0x0
                        ]),
                    }],
                }
            }
        );
    }

    #[test]
    fn test_parse_application() {
        let rsp: iso7816::SelectResponse = [
            0x6F, 0x6C, 0x84, 0x07, 0xA0, 0x00, 0x00, 0x00, 0x04, 0x10, 0x10, 0xA5, 0x61, 0x50,
            0x10, 0x44, 0x65, 0x62, 0x69, 0x74, 0x20, 0x4D, 0x61, 0x73, 0x74, 0x65, 0x72, 0x63,
            0x61, 0x72, 0x64, 0x9F, 0x12, 0x10, 0x44, 0x65, 0x62, 0x69, 0x74, 0x20, 0x4D, 0x61,
            0x73, 0x74, 0x65, 0x72, 0x63, 0x61, 0x72, 0x64, 0x87, 0x01, 0x01, 0x9F, 0x11, 0x01,
            0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x38, 0x03, 0x9F, 0x5C, 0x08, 0xBF, 0x0C,
            0x27, 0x9F, 0x5D, 0x03, 0x01, 0x00, 0x06, 0x9F, 0x0A, 0x08, 0x00, 0x01, 0x05, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x9F, 0x5E, 0x09, 0x53, 0x55, 0x22, 0x05, 0x44, 0x41, 0x72,
            0x43, 0x00, 0x9F, 0x6E, 0x07, 0x08, 0x26, 0x00, 0x00, 0x30, 0x30, 0x00,
        ][..]
            .try_into()
            .expect("couldn't parse SelectResponse");
        assert_eq!(rsp.fci.df_name, &[0xA0, 0x00, 0x00, 0x00, 0x04, 0x10, 0x10]);

        let app: Application = rsp
            .parse_into()
            .expect("couldn't parse SelectResponse into Application");
        assert_eq!(
            app,
            Application {
                app_label: "Debit Mastercard".into(),
                app_priority: Some(0x1),
                pdol: Some(vec![(0x9F5C, 0x8)]),
                lang_prefs: Some("en".into()),
                issuer_code_table_idx: Some(0x1),
                app_preferred_name: Some("Debit Mastercard".into()),
                fci_issuer_discretionary_data: Some(FCIIssuerDiscretionaryData {
                    log_entry: None,
                    app_capability_info: Some((0x01, 0x00, 0x06)),
                    app_selection_reg_propr_data: Some(vec![(
                        0x01,
                        vec![0x01, 0x00, 0x00, 0x00, 0x00]
                    )]),
                    ds_id: Some(vec![0x53, 0x55, 0x22, 0x05, 0x44, 0x41, 0x72, 0x43, 0x00]),
                    unknown_9f6e: Some(vec![0x8, 0x26, 0x0, 0x0, 0x30, 0x30, 0x0]),
                    ..Default::default()
                }),
            }
        );
    }
}
