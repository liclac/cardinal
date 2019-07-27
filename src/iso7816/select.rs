use crate::{Command, APDU, RAPDU};
use bitflags::bitflags;
use std::convert::Into;

bitflags! {
    #[derive(Default)]
    struct P1: u8 {
        /// Select by name, rather than ID.
        const SELECT_BY_NAME = 0b100;
    }
}

bitflags! {
    struct P2: u8 {
        /// Select the next occurrence. If unset, select the first occurrence.
        const NEXT_OCCURRENCE = 0b10;
    }
}

pub enum AID {
    ID(Vec<u8>),
    Name(Vec<u8>),
}

impl AID {
    fn p1(&self) -> P1 {
        match self {
            Self::ID(_) => P1::empty(),
            Self::Name(_) => P1::SELECT_BY_NAME,
        }
    }
}

impl Into<Vec<u8>> for AID {
    fn into(self) -> Vec<u8> {
        match self {
            Self::ID(v) => v,
            Self::Name(v) => v,
        }
    }
}

/// If a name has multiple matching files, what should be done for successive calls?
pub enum Occurrence {
    /// Always select the first or only file.
    First,
    /// Successive calls select successive matching files.
    Next,
}

impl Occurrence {
    fn p2(&self) -> P2 {
        match self {
            Self::First => P2::empty(),
            Self::Next => P2::NEXT_OCCURRENCE,
        }
    }
}

pub struct Select {
    pub aid: AID,
    pub occurrence: Occurrence,
}

impl Select {
    pub fn new(aid: AID) -> Self {
        Self {
            aid,
            occurrence: Occurrence::First,
        }
    }

    pub fn id<T: Into<Vec<u8>>>(id: T) -> Self {
        Self::new(AID::ID(id.into()))
    }

    pub fn name<T: Into<Vec<u8>>>(name: T) -> Self {
        Self::new(AID::Name(name.into()))
    }

    pub fn next(mut self) -> Self {
        self.occurrence = Occurrence::Next;
        self
    }
}

impl Command for Select {
    type Response = RAPDU;
}

impl Into<APDU> for Select {
    fn into(self) -> APDU {
        APDU::new(
            0x00,
            0xA4,
            self.aid.p1().bits(),
            self.occurrence.p2().bits(),
            self.aid,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn select_aid() {
        assert_eq!(
            APDU::new(
                0x00,
                0xA4,
                0x00,
                0x00,
                vec![0xA0, 0x00, 0x00, 0x00, 0x04, 0x60, 0x00]
            ),
            Select::id(vec![0xA0, 0x00, 0x00, 0x00, 0x04, 0x60, 0x00]).into(),
        );
    }

    #[test]
    fn select_aid_next() {
        assert_eq!(
            APDU::new(
                0x00,
                0xA4,
                0x00,
                0x02,
                vec![0xA0, 0x00, 0x00, 0x00, 0x04, 0x60, 0x00]
            ),
            Select::id(vec![0xA0, 0x00, 0x00, 0x00, 0x04, 0x60, 0x00])
                .next()
                .into(),
        );
    }

    #[test]
    fn select_name() {
        assert_eq!(
            APDU::new(
                0x00,
                0xA4,
                0x04,
                0x00,
                vec![
                    0x31, 0x50, 0x41, 0x59, 0x2e, 0x53, 0x59, 0x53, 0x2e, 0x44, 0x44, 0x46, 0x30,
                    0x31,
                ]
            ),
            Select::name("1PAY.SYS.DDF01".as_bytes()).into(),
        );
    }

    #[test]
    fn select_name_next() {
        assert_eq!(
            APDU::new(
                0x00,
                0xA4,
                0x04,
                0x02,
                vec![
                    0x31, 0x50, 0x41, 0x59, 0x2e, 0x53, 0x59, 0x53, 0x2e, 0x44, 0x44, 0x46, 0x30,
                    0x31,
                ]
            ),
            Select::name("1PAY.SYS.DDF01".as_bytes()).next().into(),
        );
    }
}
