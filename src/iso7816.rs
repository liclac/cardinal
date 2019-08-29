pub mod select;

pub use select::{Select, AID};

use crate::{Command, APDU, RAPDU};

pub struct GetResponse {
    pub le: u8,
}

impl GetResponse {
    pub fn new(le: u8) -> Self {
        Self { le }
    }
}

impl Command for GetResponse {
    type Response = RAPDU;
}

impl Into<APDU> for GetResponse {
    fn into(self) -> APDU {
        APDU::new(0x00, 0xC0, 0, 0, vec![]).expect(self.le)
    }
}
