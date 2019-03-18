use crate::apdu;
use crate::app::App;
use crate::ber;
use crate::card::Card;
use crate::cmd::Response;
use crate::errors::Result;
use crate::refs::FileRef;
use serde::{Deserialize, Serialize};

pub struct ADF<'a> {
    pub card: &'a Card<'a>,
    pub selection: Selection,
}

impl<'a> ADF<'a> {
    pub fn select(card: &'a Card<'a>, id: &FileRef) -> Result<Self> {
        card.select::<Self>(id)
    }
}

impl<'a> App<'a> for ADF<'a> {
    type SelectResponse = Selection;

    fn with(card: &'a Card, selection: Self::SelectResponse) -> Self {
        Self { card, selection }
    }

    fn card(&self) -> &'a Card {
        self.card
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct Selection {
    pub fci: Option<FCI>,
    pub extra: ber::Map,
}

impl Response for Selection {
    fn from_apdu(res: apdu::Response) -> Result<Self> {
        let mut sel = Self::default();
        for tvr in ber::iter(&res.data) {
            match tvr? {
                (0x6F, value) => {
                    sel.fci = Some(FCI::from_bytes(&value)?);
                }
                (tag, value) => {
                    sel.extra.insert(tag, value.into());
                }
            }
        }
        Ok(sel)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct FCI {
    pub df_name: Option<FileRef>, // Always a Name.
    pub fci_proprietary: Option<FCIProprietary>,
    pub extra: ber::Map,
}

impl FCI {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(data) {
            match tvr? {
                (0x84, value) => {
                    v.df_name = Some(FileRef::Name(value.to_vec()));
                }
                (0xA5, value) => {
                    v.fci_proprietary = Some(FCIProprietary::from_bytes(&value)?);
                }
                (tag, value) => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
pub struct FCIProprietary {
    pub app_label: Option<String>,
    pub app_priority: Option<Vec<u8>>,
    pub pdol: Option<ber::Map>,
    pub lang_pref: Option<String>,
    pub issuer_code_table_idx: Option<Vec<u8>>,
    pub app_preferred_name: Option<String>,
    pub fci_issuer_discretionary: Option<ber::Map>,
    pub extra: ber::Map,
}

impl FCIProprietary {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(data) {
            match tvr? {
                (0x50, value) => v.app_label = Some(String::from_utf8(value.into())?),
                (0x87, value) => v.app_priority = Some(value.into()),
                (0x9F38, value) => v.pdol = Some(ber::to_map(&value)?),
                (0x5F2D, value) => v.lang_pref = Some(String::from_utf8(value.into())?),
                (0x9F11, value) => v.issuer_code_table_idx = Some(value.into()),
                (0x9F12, value) => v.app_preferred_name = Some(String::from_utf8(value.into())?),
                (0xBF0C, value) => v.fci_issuer_discretionary = Some(ber::to_map(&value)?),
                (tag, value) => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}
