use crate::ber;
use crate::errors::{Error, Result};
use crate::iso7816::{RecordIter, Select, AID};
use crate::{Card, Value, RAPDU};
use serde::Serialize;
use std::collections::HashMap;
use std::convert::{From, TryFrom};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct App {
    /// 0x6F: File Control Information.
    pub fci: AppFCI,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl App {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x6f => slf.fci = AppFCI::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

impl TryFrom<RAPDU> for App {
    type Error = Error;

    fn try_from(res: RAPDU) -> Result<Self> {
        Self::parse(&res.data)
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AppFCI {
    /// 0x84: DF Name.
    pub df_name: AID,
    /// 0xA5: EMV Proprietary Data.
    pub emv: AppFCIEMV,
    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl AppFCI {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x84 => slf.df_name = AID::Name(value.into()),
                0xa5 => slf.emv = AppFCIEMV::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct AppFCIEMV {
    /// 0x50: Application Label.
    pub app_label: Option<String>,
    /// 0x87: Application Priority Indicator.
    pub app_prio: Option<Value<AppPriority>>,
    /// 0x5F2D, an2-8: Language Preference. 1-4 alpha2 ISO 639 language codes, in order of user preference.
    /// Note: EMV recommends this tag be lowercase, but uppercase should be accepted by terminals as well.
    pub lang_pref: Option<String>,
    /// 0x9F11: Issuer Code Table Index. The code page that should be used to display application labels.
    pub issuer_code_table_idx: Option<u8>,
    /// 0x9F12: Application Preferred Name.
    pub app_pref_name: Option<String>,
    /// 0xBF0C: FCI Issuer Discretionary Data. The contents are a BER-encoded map.
    pub fci_issuer: Option<HashMap<u32, Vec<u8>>>,
    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl AppFCIEMV {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x50 => slf.app_label = Some(String::from_utf8_lossy(value).into()),
                0x87 => slf.app_prio = Some(Value::new(value, AppPriority::parse(value)?)),
                0x5F2D => slf.lang_pref = Some(String::from_utf8_lossy(value).into()),
                0x9F11 => slf.issuer_code_table_idx = Some(*value.first().unwrap_or(&0)),
                0x9F12 => slf.app_pref_name = Some(String::from_utf8_lossy(value).into()),
                0xBF0C => slf.fci_issuer = Some(ber::iter(value).to_map()?),
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct Environment {
    /// 0x6F: ISO7816 File Control Information.
    pub fci: EnvironmentFCI,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl Environment {
    pub fn select() -> Select<Self> {
        Select::name("1PAY.SYS.DDF01")
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(&data) {
            let (tag, value) = tvr?;
            match tag {
                0x6F => slf.fci = EnvironmentFCI::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            };
        }
        Ok(slf)
    }

    pub fn dir_records<'a, C: Card>(&self, card: &'a C) -> RecordIter<'a, C, DirectoryRecord> {
        RecordIter::new(card, self.fci.emv.dir_sfi)
    }
}

impl TryFrom<RAPDU> for Environment {
    type Error = Error;

    fn try_from(res: RAPDU) -> Result<Self> {
        Self::parse(&res.data)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct EnvironmentFCI {
    /// 0x84, b5-16: Name of the selected file.
    pub df_name: String,

    /// 0xA5: EMV proprietary data.
    pub emv: EnvironmentFCIProprietary,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl EnvironmentFCI {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x84 => slf.df_name = String::from_utf8_lossy(value).into(),
                0xA5 => slf.emv = EnvironmentFCIProprietary::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct EnvironmentFCIProprietary {
    /// 88, n-1: SFI of the Directory Elementary File. May not exceed 30. Use in READ RECORD commands.
    pub dir_sfi: u8,

    /// 5F2D, an2-8: Language Preference. 1-4 alpha2 ISO 639 language codes, in order of user preference.
    /// Note: EMV recommends this tag be lowercase, but uppercase should be accepted by terminals as well.
    pub lang_pref: Option<String>,

    /// 9F11: Issuer Code Table Index. The code page that should be used to display application labels.
    pub issuer_code_table_idx: Option<u8>,

    /// BF0C: FCI Issuer Discretionary Data. The contents are a BER-encoded map.
    pub fci_issuer: Option<HashMap<u32, Vec<u8>>>,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl EnvironmentFCIProprietary {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x88 => slf.dir_sfi = *value.first().unwrap_or(&0),
                0x5F2D => slf.lang_pref = Some(String::from_utf8_lossy(value).into()),
                0x9F11 => slf.issuer_code_table_idx = Some(*value.first().unwrap_or(&0)),
                0xBF0C => slf.fci_issuer = Some(ber::iter(value).to_map()?),
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct DirectoryRecord {
    /// 0x70: Directory Record.
    pub record: DirectoryRecordData,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryRecord {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x70 => slf.record = DirectoryRecordData::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

impl TryFrom<RAPDU> for DirectoryRecord {
    type Error = Error;

    fn try_from(res: RAPDU) -> Result<Self> {
        Self::parse(&res.data)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct DirectoryRecordData {
    /// 0x61: Directory Entry; repeated.
    pub entries: Vec<DirectoryRecordEntry>,
    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryRecordData {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x61 => slf.entries.push(DirectoryRecordEntry::parse(value)?),
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct DirectoryRecordEntry {
    /// 0x4F: ADF Name.
    pub adf_name: AID,
    /// 0x50: Application Label.
    pub app_label: String,
    /// 0x9F12: Application Preferred Name.
    pub app_pref_name: Option<String>,
    /// 0x87: Application Priority Indicator.
    pub app_prio: Option<Value<AppPriority>>,
    /// 0x73: Directory Discretionary Template. Proprietary data from the application provider,
    ///       eg. Mastercard/Visa, chip manufacturer, or a handful of EMV-defined tags [TODO].
    pub discretionary: HashMap<u32, Vec<u8>>,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryRecordEntry {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x4F => slf.adf_name = AID::Name(value.into()),
                0x50 => slf.app_label = String::from_utf8_lossy(value).into(),
                0x9F12 => slf.app_pref_name = Some(String::from_utf8_lossy(value).into()),
                0x87 => slf.app_prio = Some(Value::new(value, AppPriority::parse(value)?)),
                0x73 => slf.discretionary = ber::iter(value).to_map()?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

/// 0x87, n-1 - Application Priority Indicator.
#[derive(Default, Debug, Serialize, PartialEq, Eq)]
pub struct AppPriority {
    /// Bit 1-4: Number in the priority list (1-15).
    pub num: u8,
    /// Bit 5-7: RFU
    pub rfu: u8,
    /// Bit 8: If set, the application may not be selected without confirmation.
    pub noauto: bool,
}

impl AppPriority {
    pub fn parse(data: &[u8]) -> Result<Self> {
        Ok(data
            .first()
            .map(|v| *v)
            .ok_or("0x87: Application Priority Indicator truncated")?
            .into())
    }

    pub fn new(num: u8, rfu: u8, noauto: bool) -> Self {
        Self { num, rfu, noauto }
    }
}

impl From<u8> for AppPriority {
    fn from(b: u8) -> Self {
        Self {
            num: b & 0b0000_1111,
            rfu: (b & 0b0111_0000) >> 4,
            noauto: b & 0b1000_0000 != 0,
        }
    }
}

impl From<AppPriority> for u8 {
    fn from(p: AppPriority) -> Self {
        p.num | p.rfu << 4 | if p.noauto { 0b1000_0000 } else { 0 }
    }
}

#[cfg(test)]
mod test_app_priority {
    use super::AppPriority;

    #[test]
    fn test_u8() {
        assert_eq!(AppPriority::new(1, 0, false), 0x01.into());
        assert_eq!(AppPriority::new(5, 0, false), 0x05.into());
        assert_eq!(AppPriority::new(15, 0, false), 0x0F.into());
        assert_eq!(AppPriority::new(12, 6, false), 0x6C.into());
        assert_eq!(AppPriority::new(12, 6, true), 0xEC.into());
    }

    #[test]
    fn test_u8_into() {
        assert_eq!(0x01u8, AppPriority::new(1, 0, false).into());
        assert_eq!(0x05u8, AppPriority::new(5, 0, false).into());
        assert_eq!(0x0Fu8, AppPriority::new(15, 0, false).into());
        assert_eq!(0x6Cu8, AppPriority::new(12, 6, false).into());
        assert_eq!(0xECu8, AppPriority::new(12, 6, true).into());
    }
}
