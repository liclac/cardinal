use crate::util;
use crate::Result;
use apdu::Command;
use pcsc::Card;

/// ID for a SELECT command.
pub enum SelectID<'a> {
    /// Select by DF name.
    Name(&'a str),
}

/// Mode for a SELECT command.
pub enum SelectMode {
    /// Select the first or only instance.
    First,
    /// Select the next instance. Can be called repeatedly.
    Next,
}

pub fn select<'r>(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &'r mut [u8],
    mode: SelectMode,
    id: SelectID,
) -> Result<&'r [u8]> {
    let cmd = Command::new_with_payload_le(
        0x00,
        0xA4,
        match id {
            SelectID::Name(_) => 0b0000_0100,
        },
        match mode {
            SelectMode::First => 0b0000_0000,
            SelectMode::Next => 0b0000_0010,
        },
        0x00,
        match id {
            SelectID::Name(name) => name.as_bytes(),
        },
    );
    util::call_apdu(card, wbuf, rbuf, cmd)
}
