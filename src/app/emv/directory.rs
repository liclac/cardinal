use crate::apdu;
use crate::app::emv::AppDef;
use crate::app::App;
use crate::ber;
use crate::card::read_record::Record;
use crate::card::Card;
use crate::cmd::Response;
use crate::errors::{Error, ErrorKind, Result};
use crate::file::FileID;
use std::collections::HashMap;

#[derive(Clone)]
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

    pub fn sfi(&self) -> Option<u8> {
        self.selection.fci_template.as_ref().and_then(|ft| {
            ft.fci_proprietary_template
                .as_ref()
                .and_then(|fpt| fpt.sfi_of_directory_ef)
        })
    }

    pub fn record_num(&self, num: u8) -> Result<Record> {
        Ok(Record::num(self.sfi().ok_or("Directory has no SFI")?, num))
    }

    pub fn records(&self) -> DirectoryRecordIterator {
        DirectoryRecordIterator::new(self)
    }
}

impl<'a> App<'a> for Directory<'a> {
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
    pub fci_template: Option<DirectoryFCIT>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl Response for DirectorySelectResponse {
    fn from_apdu(res: apdu::Response) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(&res.data) {
            let (tag, value) = tvr?;
            match tag {
                0x6F => v.fci_template = Some(DirectoryFCIT::from_bytes(value)?),
                _ => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryFCIT {
    pub df_name: Option<String>,
    pub fci_proprietary_template: Option<DirectoryFCIPropT>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryFCIT {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = DirectoryFCIT::default();
        for tvr in ber::iter(data) {
            let (tag, value) = tvr?;
            match tag {
                0x84 => v.df_name = Some(String::from_utf8(value.to_vec())?),
                0xA5 => v.fci_proprietary_template = Some(DirectoryFCIPropT::from_bytes(value)?),
                _ => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryFCIPropT {
    pub sfi_of_directory_ef: Option<u8>,
    pub lang_pref: Option<String>,
    pub issuer_code_table_idx: Option<Vec<u8>>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryFCIPropT {
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

pub struct DirectoryRecordIterator<'a> {
    dir: &'a Directory<'a>,
    num: u8,
    terminate: bool,
}

impl<'a> DirectoryRecordIterator<'a> {
    pub fn new(dir: &'a Directory<'a>) -> Self {
        DirectoryRecordIterator {
            dir,
            num: 1,
            terminate: false,
        }
    }

    fn read(&self) -> Result<DirectoryRecord> {
        self.dir.card().read_record(self.dir.record_num(self.num)?)
    }
}

impl<'a> Iterator for DirectoryRecordIterator<'a> {
    type Item = Result<DirectoryRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminate {
            return None;
        }

        let val = match self.read() {
            // Assuming all records exist in sequence, terminate the iterator on the
            // first nonexistent record. Can add a flag to not do this if not desired.
            Err(Error(ErrorKind::StatusError(apdu::Status::ErrRecordNotFound), _)) => None,
            // Terminate immediately after the first error.
            v @ Err(_) => {
                self.terminate = true;
                Some(v)
            }
            v => Some(v),
        };
        self.num += 1;
        val
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryRecord {
    pub entries: Vec<DirectoryEntry>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl Response for DirectoryRecord {
    fn from_apdu(res: apdu::Response) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(&res.data) {
            match tvr? {
                (0x70, data) => {
                    v.entries.push(DirectoryEntry::from_bytes(data)?);
                }
                (tag, value) => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    pub apps: Vec<AppDef>,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl DirectoryEntry {
    fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut v = Self::default();
        for tvr in ber::iter(&data) {
            match tvr? {
                (0x61, data) => {
                    v.apps.push(AppDef::from_bytes(data)?);
                }
                (tag, value) => {
                    v.extra.insert(tag, value.into());
                }
            };
        }
        Ok(v)
    }
}
