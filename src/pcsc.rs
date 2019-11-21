use crate::errors::{Error, ErrorKind, Result};
use crate::protocol::Protocol;
use crate::{Card as CardTrait, Context as ContextTrait, Reader as ReaderTrait, APDU, RAPDU};
use pcsc;
use std::convert::{TryFrom, TryInto};
use std::ffi::CString;
use std::fmt;
use std::rc::Rc;
use tracing::{debug, span, trace, Level};

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
    pub raw_atr: Vec<u8>,
}

impl Card {
    pub fn wrap(card: pcsc::Card) -> Result<Self> {
        let span = span!(Level::TRACE, "Card::wrap()");
        let _enter = span.enter();

        let (_status, proto) = card.status()?;
        let raw_atr = pcsc_attr(&card, pcsc::Attribute::AtrString)?;
        debug!(
            "ATR: {:}",
            raw_atr
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<String>>()
                .join("")
        );
        Ok(Self {
            card,
            proto: proto.try_into()?,
            raw_atr,
        })
    }
}

impl CardTrait for Card {
    fn exec_impl(&self, req: &APDU) -> Result<RAPDU> {
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

impl fmt::Debug for Card {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pcsc::Card {{ proto: {:#?} }}", self.proto)
    }
}

pub struct Reader {
    pub pctx: Rc<pcsc::Context>,
    pub cname: CString,
    pub name: String,
}

impl Reader {
    pub fn wrap(ctx: &Context, cname: CString) -> Self {
        Self {
            pctx: ctx.pctx.clone(),
            name: cname.to_string_lossy().into(),
            cname,
        }
    }
}

impl fmt::Display for Reader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:}", self.name)
    }
}

impl fmt::Debug for Reader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "pcsc::Reader {{ name: {:#?} }}", self.name)
    }
}

impl ReaderTrait for Reader {
    fn name(&self) -> &str {
        &self.name
    }

    fn connect(&self) -> Result<Box<dyn CardTrait>> {
        let span = span!(Level::TRACE, "Reader::connect()");
        let _enter = span.enter();

        let sharing_mode = pcsc::ShareMode::Shared;
        let protocols = pcsc::Protocols::ANY;
        trace!(
            { name = &*self.name, ?sharing_mode, ?protocols},
            "::pcsc::Context::connect()"
        );
        Ok(Box::new(Card::wrap(self.pctx.connect(
            &self.cname,
            sharing_mode,
            protocols,
        )?)?))
    }
}

pub struct Context {
    pub pctx: Rc<pcsc::Context>,
}

impl Context {
    pub fn establish(scope: pcsc::Scope) -> Result<Self> {
        let span = span!(Level::TRACE, "Context::establish()");
        let _enter = span.enter();
        trace!({ scope = "user" }, "::pcsc::Context::establish()");
        Ok(Self {
            pctx: Rc::new(pcsc::Context::establish(scope)?),
        })
    }
}

impl ContextTrait for Context {
    fn readers(&self) -> Result<Vec<Box<dyn ReaderTrait>>> {
        let span = span!(Level::TRACE, "Context::readers()");
        let _enter = span.enter();

        trace!("::pcsc::Context::list_readers_len()");
        let len = self.pctx.list_readers_len()?;
        let mut buf = Vec::with_capacity(len);
        buf.resize(buf.capacity(), 0);

        trace!({ len }, "::pcsc::Context::list_readers()");
        let names = self.pctx.list_readers(&mut buf)?;

        Ok(names
            .map(|name| Reader::wrap(&self, name.into()))
            .map(|r| Box::new(r) as Box<dyn ReaderTrait>)
            .collect())
    }
}

fn pcsc_attr(card: &pcsc::Card, attr: pcsc::Attribute) -> Result<Vec<u8>> {
    trace!({ ?attr }, "::pcsc::Card::get_attribute_len()");
    let len = card.get_attribute_len(attr)?;
    let mut buf = Vec::with_capacity(len);
    buf.resize(buf.capacity(), 0);
    trace!({ ?attr, len }, "::pcsc::Card::get_attribute()");
    card.get_attribute(attr, &mut buf[..])?;
    Ok(buf)
}
