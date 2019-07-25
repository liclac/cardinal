use crate::errors::{Error, ErrorKind, Result};
use crate::protocol::Protocol;
use crate::{Card as CardTrait, APDU, MAX_BUFFER_SIZE, RAPDU};
use pcsc;
use std::convert::{TryFrom, TryInto};

impl TryFrom<pcsc::Protocol> for Protocol {
    type Error = Error;

    fn try_from(v: pcsc::Protocol) -> std::result::Result<Self, Self::Error> {
        match v {
            pcsc::Protocol::T1 => Ok(Protocol::T1),
            pcsc::Protocol::T0 => Ok(Protocol::T0),
            pcsc::Protocol::RAW => Err(ErrorKind::UnsupportedProtocol("RAW".into()).into()),
        }
    }
}

pub struct Card {
    pub card: pcsc::Card,
    pub proto: Protocol,
}

impl Card {
    pub fn wrap(card: pcsc::Card) -> Result<Self> {
        let (_status, proto) = card.status()?;
        Ok(Self {
            card,
            proto: proto.try_into()?,
        })
    }
}

impl CardTrait for Card {
    const BUF_SIZE: usize = pcsc::MAX_BUFFER_SIZE;

    fn exec<'a>(&mut self, req: APDU<'a>, buf: &'a mut [u8]) -> Result<RAPDU<'a>> {
        let mut reqbuf = [0; MAX_BUFFER_SIZE];
        let reqlen = self.proto.write_req(&mut (&mut reqbuf[..]), &req)?;
        let req = &reqbuf[..reqlen];

        let res = self.card.transmit(&req, &mut buf[..])?;
        self.proto.decode_res(res)
    }
}
