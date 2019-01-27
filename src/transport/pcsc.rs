use crate::core::apdu;
use crate::errors::Result;
use crate::transport::protocol::{Protocol, APDU};
use crate::transport::Transport;

pub const DEFAULT_MAX_LE: usize = 256;

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
        debug!(
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
        debug!(">> SEND: RAW={:x?}", req_data);

        let mut res_buf = [0; pcsc::MAX_BUFFER_SIZE];
        let res_data = self.card.transmit(req_data.as_slice(), &mut res_buf)?;
        debug!("<< RECV: RAW={:x?}", res_data);

        let res = self.proto.deserialize_res(res_data)?;
        debug!(
            "<< RECV: SW1={:#x} SW2={:#x} DATA={:x?}",
            res.status.sw1(),
            res.status.sw2(),
            res.data
        );

        Ok(res)
    }
}
