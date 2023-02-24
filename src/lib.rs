pub mod iso7816;
pub mod probe;
mod util;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The card returned a non-standard response code (not 0x90, 0x00).
    #[error("error from card: SW1=0x{0:X} SW2=0x{1:X}")]
    APDU(u8, u8),

    #[error("expected tag {expected:#04?}, got {actual:#04?}")]
    WrongTag { expected: u32, actual: u32 },

    #[error(transparent)]
    DER(#[from] der_parser::error::Error),
    #[error(transparent)]
    DERASN1(#[from] der_parser::asn1_rs::Err<der_parser::error::Error>),

    #[error(transparent)]
    PCSC(#[from] pcsc::Error),
}
