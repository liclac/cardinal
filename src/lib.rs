pub mod atr;
pub mod ber;
pub mod emv;
pub mod felica;
pub mod iso7816;
pub mod util;

use num_enum::{FromPrimitive, IntoPrimitive};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The card returned a non-standard response code (not 0x90, 0x00).
    #[error("error from card: SW1=0x{0:02X} SW2=0x{1:02X}")]
    APDU(u8, u8),
    // Same thing, but in a PCSC Transparent Session (eg. felica::Session).
    #[error("transparent session error: DO={0:02} - {1}")]
    PCSCTransparent(u8, PCSCTransparentError),

    #[error("expected tag {expected:04X?}, got {actual:04X?}")]
    WrongTag { expected: Vec<u8>, actual: Vec<u8> },

    #[error("[felica] command failed: flag1={0:02X} flag2={1:02X}")]
    FelicaStatus(u8, u8),

    #[error("[felica] expected a {expected:?} payload, got a {actual:?}")]
    FelicaCommandCode {
        expected: felica::CommandCode,
        actual: felica::CommandCode,
    },

    #[error(transparent)]
    Scroll(#[from] scroll::Error),

    #[error(transparent)]
    Nom(#[from] nom::error::Error<HexVec>),

    #[error(transparent)]
    PCSC(#[from] pcsc::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum PCSCTransparentError {
    NoError = 0x9000,
    WarnUnavailable = 0x6282,
    NoInfo = 0x6300,
    ExecStoppedOtherDOFailed = 0x6301,
    NotSupported = 0x6A81,
    UnexpectedLength = 0x6700,
    UnexpectedValue = 0x6A80,
    NoResponseFromIFD = 0x6400,
    NoResponseFromICC = 0x6401,
    FailedUnknown = 0x6F00,
    #[num_enum(catch_all)]
    Unknown(u16) = 0x0000,
}

impl std::fmt::Display for PCSCTransparentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoError => write!(f, "{:04X} No Error", u16::from(*self)),
            Self::WarnUnavailable => write!(
                f,
                "{:04X} Warning: Requested information not available",
                u16::from(*self)
            ),
            Self::NoInfo => write!(f, "{:04X} No information", u16::from(*self)),
            Self::ExecStoppedOtherDOFailed => write!(
                f,
                "{:04X} Execution stooped due to failure in other data object",
                u16::from(*self)
            ),
            Self::NotSupported => {
                write!(f, "{:04X} Data Object not supported", u16::from(*self))
            }
            Self::UnexpectedLength => write!(
                f,
                "{:04X} Data Object has unexpected length",
                u16::from(*self)
            ),
            Self::UnexpectedValue => {
                write!(
                    f,
                    "{:04X} Data Object has unexpected value",
                    u16::from(*self)
                )
            }
            Self::NoResponseFromIFD => {
                write!(
                    f,
                    "{:04X} Data Object execution error: No response from IFD",
                    u16::from(*self)
                )
            }
            Self::NoResponseFromICC => {
                write!(
                    f,
                    "{:04X} Data Object execution error: No response from ICC",
                    u16::from(*self)
                )
            }
            Self::FailedUnknown => write!(
                f,
                "{:04X} Data object failed, no precise diagnosis",
                u16::from(*self)
            ),
            Self::Unknown(v) => write!(f, "{:04X} Unknown Error", v),
        }
    }
}

impl From<nom::error::Error<&[u8]>> for Error {
    fn from(value: nom::error::Error<&[u8]>) -> Self {
        Self::Nom(nom::error::Error::new(
            HexVec(value.input.into()),
            value.code,
        ))
    }
}

impl From<nom::Err<nom::error::Error<&[u8]>>> for Error {
    fn from(value: nom::Err<nom::error::Error<&[u8]>>) -> Self {
        match value {
            nom::Err::Error(err) => err.into(),
            nom::Err::Failure(err) => err.into(),
            nom::Err::Incomplete(_) => {
                panic!("can't convert nom::Err::Incomplete into cardinal::Error")
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct HexVec(pub Vec<u8>);

impl std::fmt::Display for HexVec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02X?}", self.0)
    }
}
