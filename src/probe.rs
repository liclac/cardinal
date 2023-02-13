use pcsc::Card;
use tracing::{trace_span, warn};

use crate::util::call_le;
use crate::Result;

#[derive(Debug)]
pub struct Probe {
    /// [Contactless Only] Card ID.
    /// Transmitted as part of an ISO 14443-4 Answer-to-Select.
    pub cid: Option<Vec<u8>>,
}

pub fn probe(card: &mut Card) -> Result<Probe> {
    let span = trace_span!("probe");
    let _enter = span.enter();

    let mut wbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Request buffer.
    let mut rbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Response buffer.
    Ok(Probe {
        cid: probe_cid(card, &mut wbuf, &mut rbuf),
    })
}

fn probe_cid(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8]) -> Option<Vec<u8>> {
    let span = trace_span!("probe_cid");
    let _enter = span.enter();

    // This pseudo-APDU is defined by the PCSC standard.
    call_le(card, wbuf, rbuf, 0xFF, 0xCA, 0x00, 0x00, 0)
        .map_err(|err| {
            warn!("couldn't query CID: {}", err);
            err
        })
        .map(|data| data.into())
        .ok()
}
