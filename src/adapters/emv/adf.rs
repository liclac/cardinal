use crate::ber;
use crate::core::FileID;
use crate::errors::Result;
use std::collections::HashMap;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AppDef {
    pub adf_id: Option<FileID>, // Always a Name.
    pub app_label: Option<String>,
    pub app_preferred_name: Option<String>,
    pub app_priority: Vec<u8>,
    pub dir_dicretionary_data: ber::Map,
    pub extra: HashMap<u32, Vec<u8>>,
}

impl AppDef {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut def = AppDef::default();
        for tvr in ber::iter(&data) {
            match tvr? {
                (0x4F, value) => {
                    def.adf_id = Some(FileID::Name(value.to_vec()));
                }
                (0x50, value) => {
                    def.app_label = Some(String::from_utf8(value.to_vec())?);
                }
                (0x9F12, value) => {
                    def.app_preferred_name = Some(String::from_utf8(value.to_vec())?);
                }
                (0x87, value) => {
                    def.app_priority = value.to_vec();
                }
                (0x73, value) => {
                    def.dir_dicretionary_data = ber::to_map(value)?;
                }
                (tag, value) => {
                    def.extra.insert(tag, value.into());
                }
            }
        }
        Ok(def)
    }
}
