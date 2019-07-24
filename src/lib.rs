pub mod errors;
pub mod pcsc;
pub mod protocol;
pub mod util;

use crate::errors::Result;

/// Max buffer size for an APDU or R-APDU, in any protocol.
const MAX_BUFFER_SIZE: usize = 264;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct APDU<'a> {
    /// Class- and instruction bytes. The instruction depends on the application,
    /// the class on the application and context (eg. secure messaging).
    pub cla: u8,
    pub ins: u8,

    /// Arguments to the command. Some commands use these, others just use data.
    pub p1: u8,
    pub p2: u8,

    /// Command data! The length field is set automatically by the transport.
    pub data: &'a [u8],

    /// Expected response length, where 0 = 256. Mismatches between the APDU's Le and
    /// the R-APDU's length will be handled automatically by the Card trait.
    pub le: u8,
}

impl<'a> APDU<'a> {
    pub fn new(cla: u8, ins: u8, p1: u8, p2: u8, data: &'a [u8]) -> Self {
        Self {
            cla,
            ins,
            p1,
            p2,
            data,
            le: 0,
        }
    }

    pub fn expect(mut self, le: u8) -> Self {
        self.le = le;
        self
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct RAPDU<'a> {
    /// Response data.
    pub data: &'a [u8],
    // Status word; (0x90, 0x00) is success.
    pub sw: StatusCode,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct StatusCode(pub u8, pub u8);

/// A higher-level interface around a smartcard reader.
pub trait Card {
    const BUF_SIZE: usize;

    /// Executes an APDU against the card, and returns the response.
    /// The response will be read into buf, which must be at least BUF_SIZE in length.
    fn exec<'a>(&mut self, req: APDU<'a>, buf: &'a mut [u8]) -> Result<RAPDU<'a>>;
}
