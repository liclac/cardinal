use crate::card::Card;
use crate::core::apdu;
use crate::errors::Result;
use crate::transport::protocol::APDU;
use crate::transport::Transport;

pub const DEFAULT_MAX_LE: usize = 256;

pub struct PCSC {
    pub card: pcsc::Card,

    // TODO: Add a way to query this from the card; ATR might have it???
    pub max_le: usize,
}

impl PCSC {
    pub fn new(card: pcsc::Card) -> Self {
        Self {
            card,
            max_le: DEFAULT_MAX_LE,
        }
    }
}

impl Card for PCSC {}

impl Transport for PCSC {
    type Protocol = APDU;

    fn call_raw_apdu(&self, req: &apdu::Request) -> Result<apdu::Response> {
        // The Le (expected response length) is typically auto-detected, but can be overridden.
        let le = match req.le {
            Some(v) => v,
            None => 256,
        };

        info!(
            ">> cla={:x} ins={:x} p1={:x} p2={:x} Lc={:} Le={:}",
            req.cla,
            req.ins,
            req.p1,
            req.p2,
            req.data.len(),
            le,
        );

        let req_vec = self.serialize_req(req)?;
        let mut res_buf = [0; pcsc::MAX_BUFFER_SIZE];
        let res_data = self.card.transmit(req_vec.as_slice(), &mut res_buf)?;
        self.deserialize_res(res_data)
    }
}
