pub mod ber;
pub mod emv;
pub mod errors;
pub mod iso7816;
pub mod pcsc;
pub mod protocol;
pub mod util;

use crate::errors::{Error, ErrorKind, Result};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RAPDU {
    /// Response data.
    pub data: Vec<u8>,
    // Status word; (0x90, 0x00) is success.
    pub sw: Status,
}

impl From<RAPDU> for () {
    fn from(_: RAPDU) -> Self {
        ()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 0x9000: OK. Any other 0x90XX is RFU and will be parsed as Unknown(0x90, xx).
    OK,

    /// 0x61XX: Instructs the caller to issue a GET RESPONSE command with Le=xx.
    ///
    /// This is a "procedure byte" and, along with RetryWithLe(xx), deals with the fact that the length of a
    /// response can't always be known ahead of time.
    ///
    /// This is normally handled automatically, unless you're making manual calls to Card::exec().
    GetResponse(u8),

    /// 0x6Cxx: Instructs the caller to retry the last command with Le=xx.
    ///
    /// This is a "procedure byte" and, along with GetResponse(xx), deals with the fact that the length of a
    /// response can't always be known ahead of time.
    ///
    /// This is normally handled automatically, unless you're making manual calls to Card::exec().
    RetryWithLe(u8),

    /// 0x6283: State of non-volatile memory unchanged; selected file invalidated.
    ///
    /// TODO: Figure out what this actually means.
    SelectionInvalidated,

    /// 0x6300: State of non-volatile memory changed; authentication failed.
    AuthenticationFailed,

    /// 0x63CX: State of non-volatile memory changed; counter provided by 'x' (0-15).
    ///
    /// This is used for eg. the number of attempted PIN entries for EMV payment cards.
    Counter(u8),

    /// 0x6983: Command not allowed; authentication method blocked.
    AuthMethodBlocked,
    /// 0x6984 Command not allowed; referenced data invalidated
    DataInvalidated,
    /// 0x6985 Command not allowed; conditions of use not satisfied.
    ConditionsOfUse,

    /// 0x6A81 Wrong parameter(s) P1 P2; function not supported.
    FunctionNotSupported,
    /// 0x6A82 Wrong parameter(s) P1 P2; file not found.
    FileNotFound,
    /// 0x6A83 Wrong parameter(s) P1 P2; record not found.
    RecordNotFound,
    /// 0x6A86 Wrong parameter(s) P1 P2.
    WrongP1P2,
    /// 0x6A88 Referenced data (data objects) not found.
    DataNotFound,

    /// We've encountered something we don't understand.
    Unknown(u8, u8),
}

impl Status {
    pub fn from(sw1: u8, sw2: u8) -> Self {
        match (sw1, sw2) {
            (0x90, 0x00) => Self::OK,
            (0x61, xx) => Self::GetResponse(xx),
            (0x6C, xx) => Self::RetryWithLe(xx),
            (0x63, xx) => {
                if xx < 0xC0 {
                    Self::Unknown(0x63, xx)
                } else {
                    Self::Counter(xx)
                }
            }
            (0x69, 0x83) => Self::AuthMethodBlocked,
            (0x69, 0x84) => Self::DataInvalidated,
            (0x69, 0x85) => Self::ConditionsOfUse,
            (0x6A, 0x81) => Self::FunctionNotSupported,
            (0x6A, 0x82) => Self::FileNotFound,
            (0x6A, 0x83) => Self::RecordNotFound,
            (0x6A, 0x86) => Self::WrongP1P2,
            (0x6A, 0x88) => Self::DataNotFound,
            (x, y) => Self::Unknown(x, y),
        }
    }
}

/// Higher-level trait for card commands.
pub trait Command: Into<APDU> {
    type Response: TryFrom<RAPDU>;
}

impl Command for APDU {
    type Response = RAPDU;
}

/// A higher-level interface around a smartcard reader.
pub trait Card: std::fmt::Debug {
    fn exec_impl(&self, req: &APDU) -> Result<RAPDU>;

    /// Executes an APDU against the card, and returns the response.
    fn exec(&self, req: APDU) -> Result<RAPDU> {
        let resp = self.exec_impl(&req)?;
        match resp.sw {
            Status::OK => Ok(resp),
            Status::GetResponse(xx) => self.exec(iso7816::GetResponse::new(xx).into()),
            Status::RetryWithLe(xx) => self.exec(req.expect(xx)),
            x => Err(ErrorKind::APDU(x).into()),
        }
    }

    /// Executes a command against the card, and returns the response.
    fn call<C: Command>(&self, req: C) -> Result<C::Response>
    where
        Error: From<<C::Response as TryFrom<RAPDU>>::Error>,
    {
        Ok(self.exec(req.into())?.try_into()?)
    }
}
