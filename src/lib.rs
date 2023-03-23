pub mod atr;
pub mod ber;
pub mod emv;
pub mod felica;
pub mod iso7816;
pub mod probe;
mod util;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The card returned a non-standard response code (not 0x90, 0x00).
    #[error("error from card: SW1=0x{0:X} SW2=0x{1:X}")]
    APDU(u8, u8),

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
