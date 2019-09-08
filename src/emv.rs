use crate::ber;
use crate::errors::{Error, Result};
use crate::iso7816::{RecordIter, Select};
use crate::{Card, RAPDU};
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct Environment<'a, C: Card> {
    pub card: &'a C,
    pub data: EnvironmentData,
}

impl<'a, C: Card> Environment<'a, C> {
    pub fn new(card: &'a C) -> Self {
        Self {
            card,
            data: EnvironmentData::default(),
        }
    }

    pub fn select(mut self) -> Result<Self> {
        self.data = self.card.call(Select::name("1PAY.SYS.DDF01"))?;
        Ok(self)
    }

    pub fn dir_records(&self) -> RecordIter<'a, C, DirectoryRecord> {
        RecordIter::new(self.card, self.data.fci.fci_proprietary.dir_sfi)
    }
}

#[derive(Debug, Default)]
pub struct EnvironmentData {
    /// 0x6F: ISO7816 File Control Information.
    pub fci: EnvironmentFCI,

    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl TryFrom<RAPDU> for EnvironmentData {
    type Error = Error;

    fn try_from(res: RAPDU) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(&res.data) {
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
}

#[derive(Debug, Default)]
pub struct EnvironmentFCI {
    /// 0x84, b5-16: Name of the selected file.
    pub df_name: String,

    /// 0xA5: EMV proprietary data.
    pub fci_proprietary: EnvironmentFCIProprietary,

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
                0xA5 => slf.fci_proprietary = EnvironmentFCIProprietary::parse(value)?,
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
pub struct DirectoryRecord {
    /// Unknown tags.
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryRecord {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
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
