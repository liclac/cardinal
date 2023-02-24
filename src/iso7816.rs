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

pub struct Select<'a> {
    pub id: SelectID<'a>,
    pub mode: SelectMode,
}

impl<'a> Select<'a> {
    pub fn exec<'r>(
        self,
        card: &mut Card,
        wbuf: &mut [u8],
        rbuf: &'r mut [u8],
    ) -> Result<&'r [u8]> {
        util::call_apdu(card, wbuf, rbuf, self.into())
    }
}

impl<'a> From<Select<'a>> for Command<'a> {
    fn from(v: Select<'a>) -> Self {
        Self::new_with_payload_le(
            0x00,
            0xA4,
            match v.id {
                SelectID::Name(_) => 0b0000_0100,
            },
            match v.mode {
                SelectMode::First => 0b0000_0000,
                SelectMode::Next => 0b0000_0010,
            },
            0x00,
            match v.id {
                SelectID::Name(name) => name.as_bytes(),
            },
        )
    }
}

/// Response type for a SELECT command.
/// Also known as an FCI (File Control Information) struct.
pub struct SelectResponse<'a> {
    /// 0x84 DF Name.
    pub df_name: &'a [u8],

    /// 0xA5 FCI Proprietary Template (the contents of the file).
    /// Contents and encoding depend on the file selected.
    ///
    /// Selecting a file and reading the contents of said file is the single most common
    /// operation there is, so I'm not typing `.fci_proprietary_template` every time.
    pub pt: &'a [u8],
}

impl<'a> TryFrom<&'a [u8]> for SelectResponse<'a> {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let (_, obj) = der_parser::ber::parse_ber(data)?;
        util::expect_tag(0x6F, obj.header.raw_tag())?;
        Ok(Self {
            df_name: data,
            pt: data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_select_response() {
        SelectResponse::try_from(
            &[
                0x6F, 0x1E, 0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44,
                0x44, 0x46, 0x30, 0x31, 0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E,
                0x9F, 0x11, 0x01, 0x01,
            ][..],
        )
        .expect("couldn't parse SelectResponse");
    }
}
