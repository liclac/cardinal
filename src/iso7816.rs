use crate::{ber, util, Result};
use apdu::Command;
use pcsc::Card;
use tracing::{trace_span, warn};

pub fn select_name<'r, R: TryFrom<&'r [u8]>>(
    card: &mut Card,
    wbuf: &mut [u8],
    rbuf: &'r mut [u8],
    name: &[u8],
) -> Result<R, R::Error>
where
    R::Error: From<crate::Error>,
{
    Select {
        id: SelectID::Name(name),
        mode: SelectMode::First,
    }
    .call(card, wbuf, rbuf)?
    .parse_into()
}

/// ID for a SELECT command.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectID<'a> {
    /// Select by DF name.
    Name(&'a [u8]),
}

/// Mode for a SELECT command.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectMode {
    /// Select the first or only instance.
    First,
    /// Select the next instance. Can be called repeatedly.
    Next,
}

#[derive(Debug, PartialEq, Eq)]
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

    pub fn call<'r>(
        self,
        card: &mut Card,
        wbuf: &mut [u8],
        rbuf: &'r mut [u8],
    ) -> Result<SelectResponse<'r>> {
        self.exec(card, wbuf, rbuf)?.try_into()
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
                SelectID::Name(name) => name,
            },
        )
    }
}

/// Response type for a SELECT command.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct SelectResponse<'a> {
    /// 0x6F File Control Information. Describes the selected file.
    pub fci: FileControlInfo<'a>,
}

impl<'a> SelectResponse<'a> {
    /// Parses a SELECT response's FCI Proprietary Template data into a domain-specific type.
    pub fn parse_into<R: TryFrom<&'a [u8]>>(&self) -> Result<R, R::Error>
    where
        R::Error: From<crate::Error>,
    {
        R::try_from(self.fci.pt.unwrap_or_default().into())
    }
}

impl<'a> TryFrom<&'a [u8]> for SelectResponse<'a> {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        let span = trace_span!("SelectResponse");
        let _enter = span.enter();

        let (_, (tag, value)) = ber::parse_next(data)?;
        util::expect_tag(&[0x6F], tag)?;

        Ok(Self {
            fci: value.try_into()?,
        })
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileControlInfo<'a> {
    /// 0x84 DF Name. (Required)
    pub df_name: &'a [u8],

    /// 0xA5 FCI Proprietary Template (the contents of the file).
    /// Contents and encoding depend on the file selected.
    ///
    /// Selecting a file and reading the contents of said file is the single most common
    /// operation there is, so I'm not typing `.fci_proprietary_template` every time.
    pub pt: Option<&'a [u8]>,
}

impl<'a> TryFrom<&'a [u8]> for FileControlInfo<'a> {
    type Error = crate::Error;

    fn try_from(data: &'a [u8]) -> Result<Self> {
        let span = trace_span!("FileControlInfo");
        let _enter = span.enter();

        let mut slf = Self::default();
        for res in ber::iter(data) {
            let (tag, value) = res?;
            match tag {
                &[0x84] => slf.df_name = value,
                &[0xA5] => slf.pt = Some(value),
                _ => warn!("FileControlInfo contains unknown field: {:X?}", tag),
            }
        }

        Ok(slf)
    }
}

/// ID for a READ RECORD command.
#[derive(Debug, PartialEq, Eq)]
pub enum RecordID {
    /// Select by DF name.
    Number(u8),
}

// A READ RECORD command.
#[derive(Debug, PartialEq, Eq)]
pub struct ReadRecord {
    pub sfi: u8,
    pub id: RecordID,
}

impl ReadRecord {
    pub fn exec<'r>(
        self,
        card: &mut Card,
        wbuf: &mut [u8],
        rbuf: &'r mut [u8],
    ) -> Result<&'r [u8]> {
        util::call_apdu(card, wbuf, rbuf, self.into())
    }

    pub fn call<'r>(
        self,
        card: &mut Card,
        wbuf: &mut [u8],
        rbuf: &'r mut [u8],
    ) -> Result<ReadRecordResponse<'r>> {
        Ok(self.exec(card, wbuf, rbuf)?.into())
    }
}

impl<'a> From<ReadRecord> for Command<'a> {
    fn from(v: ReadRecord) -> Self {
        Self::new_with_le(
            0x00,
            0xB2,
            match v.id {
                RecordID::Number(num) => num,
            },
            (v.sfi << 3)
                | match v.id {
                    RecordID::Number(_) => 0b0000_0100,
                },
            0x00,
        )
    }
}

/// Response type for a READ RECORD command.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ReadRecordResponse<'a> {
    pub data: &'a [u8],
}

impl<'a> ReadRecordResponse<'a> {
    pub fn parse_into<R: TryFrom<&'a [u8]>>(&self) -> Result<R, R::Error>
    where
        R::Error: From<crate::Error>,
    {
        R::try_from(self.data.into())
    }
}

impl<'a> From<&'a [u8]> for ReadRecordResponse<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self { data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_response_parse_emv_dir() {
        let rsp: SelectResponse = [
            0x6F, 0x1E, 0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44,
            0x44, 0x46, 0x30, 0x31, 0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E,
            0x9F, 0x11, 0x01, 0x01,
        ][..]
            .try_into()
            .expect("couldn't parse SelectResponse");
        assert_eq!(rsp.fci.df_name, "1PAY.SYS.DDF01".as_bytes());
        assert_eq!(
            rsp.fci.pt,
            Some(&[0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01][..])
        );
    }

    #[test]
    fn test_apdu_read_record() {
        let c: apdu::Command = (ReadRecord {
            sfi: 1,
            id: RecordID::Number(1),
        })
        .into();
        let mut buf = [0u8; 256];
        c.write(&mut buf[..]);
        assert_eq!(&buf[..c.len()], &[0x00, 0xB2, 0x01, 0x0C, 0x00]);
    }
}
