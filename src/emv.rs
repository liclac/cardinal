use crate::ber;
use crate::errors::{Error, Result};
use crate::iso7816::Select;
use crate::{Card, RAPDU};
use std::collections::HashMap;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct Environment<C: Card> {
    pub card: C,
    pub data: EnvironmentData,
}

impl<C: Card> Environment<C> {
    pub fn new(card: C) -> Self {
        Self {
            card,
            data: EnvironmentData::default(),
        }
    }

    pub fn select(mut self) -> Result<Self> {
        self.data = self.card.call(Select::name("1PAY.SYS.DDF01"))?;
        Ok(self)
    }
}

#[derive(Debug, Default)]
pub struct EnvironmentData {
    pub fci: EnvironmentFCI,
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
    pub df_name: String,
    pub fci_proprietary: EnvironmentFCIProprietary,
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
    pub lang_pref: Option<String>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl EnvironmentFCIProprietary {
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut slf = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x5f2d => slf.lang_pref = Some(String::from_utf8_lossy(value).into()),
                _ => {
                    slf.extra.insert(tag, value.into());
                }
            }
        }
        Ok(slf)
    }
}
