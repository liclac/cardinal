use crate::ber;
use crate::card::card::Card;
use crate::card::commands::Record;
use crate::card::interface::Interface;
use crate::core::apdu;
use crate::core::command::Response;
use crate::core::file::FileID;
use crate::errors::Result;
use std::collections::HashMap;

pub struct Directory<'a> {
    pub card: &'a Card<'a>,
    pub selection: DirectorySelectResponse,
}

impl<'a> Directory<'a> {
    pub fn id() -> FileID {
        FileID::Name("1PAY.SYS.DDF01".into())
    }

    pub fn select(card: &'a Card<'a>) -> Result<Self> {
        card.select::<Self>(&Self::id())
    }

    pub fn record_num(&self, num: u8) -> Result<Record> {
        Ok(Record::num(
            self.selection
                .fci_template
                .as_ref()
                .ok_or("EMV directory has no FCI Template")?
                .fci_proprietary_template
                .as_ref()
                .ok_or("FCI Template has no FCI Proprietary Template")?
                .sfi_of_directory_ef
                .ok_or("FCI Proprietary Template has no Directory SFI")?,
            num,
        ))
    }
}

impl<'a> Interface<'a> for Directory<'a> {
    type SelectResponse = DirectorySelectResponse;

    fn with(card: &'a Card<'a>, selection: Self::SelectResponse) -> Self {
        Self { card, selection }
    }

    fn card(&self) -> &'a Card {
        self.card
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectorySelectResponse {
    pub fci_template: Option<FCITemplate>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl Response for DirectorySelectResponse {
    fn from_apdu(res: apdu::Response) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(&res.data) {
            let (tag, value) = tvr?;
            match tag {
                0x6F => v.fci_template = Some(FCITemplate::from_bytes(value)?),
                _ => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FCITemplate {
    pub df_name: Option<String>,
    pub fci_proprietary_template: Option<FCIProprietaryTemplate>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl FCITemplate {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = FCITemplate::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x84 => v.df_name = Some(String::from_utf8(value.to_vec())?),
                0xA5 => {
                    v.fci_proprietary_template = Some(FCIProprietaryTemplate::from_bytes(value)?)
                }
                _ => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FCIProprietaryTemplate {
    pub sfi_of_directory_ef: Option<u8>,
    pub lang_pref: Option<String>,
    pub issuer_code_table_idx: Option<Vec<u8>>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl FCIProprietaryTemplate {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x88 => v.sfi_of_directory_ef = value.first().cloned(),
                0x5F2D => v.lang_pref = Some(String::from_utf8(value.to_vec())?),
                0x9F11 => v.issuer_code_table_idx = Some(value.into()),
                _ => {
                    v.extra.insert(tag, value.into());
                }
            }
        }
        Ok(v)
    }
}
