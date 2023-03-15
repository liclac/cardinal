//! Interfaces to EMV payment cards.
//!
//! The EMV specifications can be downloaded freely from EMVCo's website, and
//! also contain a recap of ISO 7816.
//!
//! All data elements are defined in Book 1, Annex B.

use crate::{ber, iso7816, Result};
use pcsc::Card;
use tracing::warn;

pub const DIRECTORY_DF_NAME: &str = "1PAY.SYS.DDF01";

/// The EMV Directory, also known as the Payment System Environment.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Directory {
    /// 0x88: SFI of the Directory Elementary File. (Values 1-30.)
    pub ef_sfi: u8,

    /// 0x5F2D: Language Preference. (an2, 2-8 bytes)
    /// List of 2-character language codes, eg. "enfr" (English, French).
    pub lang_prefs: Option<String>,
}

impl<'a> Directory {
    pub fn select(card: &mut Card, wbuf: &mut [u8], rbuf: &'a mut [u8]) -> Result<Self> {
        iso7816::select_name(card, wbuf, rbuf, DIRECTORY_DF_NAME)
    }
}

impl<'a> TryFrom<&'a [u8]> for Directory {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self> {
        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x88] => {
                    slf.ef_sfi = *value.first().unwrap_or_else(|| {
                        warn!("0x88 ef_sfi is empty!");
                        &0
                    })
                }
                &[0x5F, 0x2D] => slf.lang_prefs = Some(String::from_utf8_lossy(value).into()),
                _ => warn!("EMV Directory contains unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_directory_selection() {
        // `SELECT '1PAY.SYS.DDF01'` response from a (Nitecrest) Monzo card.
        let rsp: iso7816::SelectResponse = [
            0x6F, 0x1E, 0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44,
            0x44, 0x46, 0x30, 0x31, 0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E,
            0x9F, 0x11, 0x01, 0x01,
        ][..]
            .try_into()
            .expect("couldn't parse SelectResponse");
        assert_eq!(rsp.fci.df_name, "1PAY.SYS.DDF01".as_bytes());
        assert_eq!(
            rsp.fci.pt,
            Some(&[0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01][..])
        );

        let dir: Directory = rsp
            .parse_into()
            .expect("couldn't parse SelectResponse into Directory");
        assert_eq!(
            dir,
            Directory {
                ef_sfi: 1,
                lang_prefs: Some("en".into()),
            }
        );
    }
}
