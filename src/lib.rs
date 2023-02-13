pub mod probe;
mod util;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The card returned a non-standard response code (not 0x90, 0x00).
    #[error("error from card: SW1=0x{0:X} SW2=0x{1:X}")]
    APDU(u8, u8),

    #[error(transparent)]
    PCSC(#[from] pcsc::Error),
}
