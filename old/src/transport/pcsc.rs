use crate::apdu;
use crate::errors::Result;
use crate::transport::protocol::{Protocol, APDU};
use crate::transport::Transport;
use log::trace;
use std::ffi::CString;
use std::rc::Rc;

pub const DEFAULT_MAX_LE: usize = 256;

pub struct Reader {
    pctx: Rc<pcsc::Context>,
    c_name: CString,

    pub name: String,
}

impl Reader {
    pub fn list() -> Result<Vec<Reader>> {
        let pctx = Rc::new(pcsc::Context::establish(pcsc::Scope::User)?);
        let mut buf = Vec::with_capacity(pctx.list_readers_len()?);
        buf.resize(buf.capacity(), 0);
        let mut readers = Vec::new();
        for name in pctx.list_readers(&mut buf)? {
            readers.push(Reader {
                pctx: pctx.clone(),
                c_name: name.into(),
                name: name.to_str()?.into(),
            })
        }
        Ok(readers)
    }

    pub fn connect(&self) -> Result<PCSC> {
        Ok(PCSC::new(self.pctx.connect(
            self.c_name.as_c_str(),
            pcsc::ShareMode::Shared,
            pcsc::Protocols::ANY,
        )?))
    }
}

pub struct PCSC {
    pub card: pcsc::Card,
    pub proto: APDU,

    // TODO: Add a way to query this from the card; ATR might have it???
    pub max_le: usize,
}

impl PCSC {
    pub fn new(card: pcsc::Card) -> Self {
        Self {
            card,
            proto: APDU::new(),
            max_le: DEFAULT_MAX_LE,
        }
    }
}

impl Transport for PCSC {
    fn call_raw_apdu(&self, req: &apdu::Request) -> Result<apdu::Response> {
        // The Le (expected response length) is typically auto-detected, but can be overridden.
        let le = match req.le {
            Some(v) => v,
            None => 256,
        };
        trace!(
            ">> SEND: CLA={:#x} INS={:#x} P1={:#x} P2={:#x} Lc={:} Le={:} DATA={:x?}",
            req.cla,
            req.ins,
            req.p1,
            req.p2,
            req.data.len(),
            le,
            req.data,
        );

        let req_data = self.proto.serialize_req(req)?;
        trace!(">> SEND: RAW={:x?}", req_data);

        let mut res_buf = [0; pcsc::MAX_BUFFER_SIZE];
        let res_data = self.card.transmit(req_data.as_slice(), &mut res_buf)?;
        trace!("<< RECV: RAW={:x?}", res_data);

        let res = self.proto.deserialize_res(res_data)?;
        trace!(
            "<< RECV: SW1={:#x} SW2={:#x} DATA={:x?}",
            res.status.sw1(),
            res.status.sw2(),
            res.data
        );

        Ok(res)
    }
}
