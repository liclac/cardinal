pub mod ber;
pub mod errors;
pub mod iso7816;
pub mod pcsc;
pub mod protocol;
pub mod util;

use crate::errors::{Error, Result};
use std::convert::{TryFrom, TryInto};

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct APDU {
    /// Class- and instruction bytes. The instruction depends on the application,
    /// the class on the application and context (eg. secure messaging).
    pub cla: u8,
    pub ins: u8,

    /// Arguments to the command. Some commands use these, others just use data.
    pub p1: u8,
    pub p2: u8,

    /// Command data! The length field is set automatically by the transport.
    pub data: Vec<u8>,

    /// Expected response length, where 0 = 256. Mismatches between the APDU's Le and
    /// the R-APDU's length will be handled automatically by the Card trait.
    pub le: u8,
}

impl APDU {
    pub fn new<D: Into<Vec<u8>>>(cla: u8, ins: u8, p1: u8, p2: u8, data: D) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data: data.into(),
            le: 0,
        }
    }

    pub fn expect(mut self, le: u8) -> Self {
        self.le = le;
        self
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct RAPDU {
    /// Response data.
    pub data: Vec<u8>,
    // Status word; (0x90, 0x00) is success.
    pub sw: StatusCode,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct StatusCode(pub u8, pub u8);

/// Higher-level trait for card commands.
pub trait Command: TryInto<APDU> {
    type Response: TryFrom<RAPDU>;
}

impl Command for APDU {
    type Response = RAPDU;
}

/// A higher-level interface around a smartcard reader.
pub trait Card {
    /// Executes an APDU against the card, and returns the response.
    /// This is a low-level function and does not necessarily handle Le.
    fn exec(&mut self, req: &APDU) -> Result<RAPDU>;

    /// Executes a command against the card, and returns the response.
    ///
    /// TODO: Handle Le with retries or GET RESPONSE.
    fn call<C: Command>(&mut self, req: C) -> Result<C::Response>
    where
        Error: From<C::Error> + From<<C::Response as TryFrom<RAPDU>>::Error>,
    {
        Ok(self.exec(&req.try_into()?)?.try_into()?)
    }
}
