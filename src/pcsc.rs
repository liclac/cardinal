use crate::errors::{Error, ErrorKind, Result};
use crate::protocol::Protocol;
use crate::{Card as CardTrait, APDU, RAPDU};
use pcsc;
use std::convert::{TryFrom, TryInto};
use tracing::trace;

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
    fn exec_impl(&mut self, req: &APDU) -> Result<RAPDU> {
        let mut reqbuf = [0; pcsc::MAX_BUFFER_SIZE];
        let reqlen = self.proto.write_req(&mut (&mut reqbuf[..]), &req)?;
        let reqdata = &reqbuf[..reqlen];
        trace!(">> {:02x?}", reqdata);

        let mut resbuf = [0; pcsc::MAX_BUFFER_SIZE];
        let resdata = self.card.transmit(&reqdata, &mut resbuf[..])?;
        trace!("<< {:02x?}", resdata);

        self.proto.decode_res(&resdata)
    }
}

impl std::fmt::Debug for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "pcsc::Card {{ proto: {:#?} }}", self.proto)
    }
}
