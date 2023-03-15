use pcsc::Card;
use tracing::{debug, trace_span, warn};

use crate::util::call_le;
use crate::{emv, Result};

#[derive(Debug)]
pub struct Probe {
    /// [Contactless Only] Card ID.
    /// Transmitted as part of an ISO 14443-4 Answer-to-Select.
    pub cid: Option<Vec<u8>>,

    /// Answer-to-Reset (or Answer-to-Select for contactless) blob.
    pub atr: Option<Vec<u8>>,

    /// Reader attributes (from PCSC).
    pub reader: ReaderAttrs,

    /// EMV payment system data.
    pub emv: Option<EMV>,
}

#[derive(Debug)]
pub struct ReaderAttrs {
    pub vendor_name: Option<String>,
    pub friendly_name: Option<String>,
    pub system_name: Option<String>,
}

pub fn probe(card: &mut Card) -> Result<Probe> {
    let span = trace_span!("probe");
    let _enter = span.enter();

    let mut wbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Request buffer.
    let mut rbuf = [0; pcsc::MAX_BUFFER_SIZE]; // Response buffer.
    Ok(Probe {
        cid: probe_cid(card, &mut wbuf, &mut rbuf),
        atr: probe_atr(card),
        reader: probe_reader_attrs(card, &mut rbuf),
        emv: EMV::probe(card, &mut wbuf, &mut rbuf),
    })
}

fn probe_cid(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8]) -> Option<Vec<u8>> {
    let span = trace_span!("probe_cid");
    let _enter = span.enter();

    // This pseudo-APDU is defined by the PCSC standard.
    let cid = call_le(card, wbuf, rbuf, 0xFF, 0xCA, 0x00, 0x00, 0)
        .map_err(|err| {
            warn!("couldn't query CID: {}", err);
            err
        })
        .map(|data| data.into())
        .ok();
    if let Some(ref cid) = cid {
        debug!("CID: {:02X?}", cid);
    }
    cid
}

fn probe_atr(card: &mut Card) -> Option<Vec<u8>> {
    let span = trace_span!("probe_atr");
    let _enter = span.enter();

    card.get_attribute_owned(pcsc::Attribute::AtrString)
        .map_err(|err| {
            warn!("couldn't query ATR: {}", err);
            err
        })
        .ok()
}

fn probe_reader_attrs(card: &mut Card, rbuf: &mut [u8]) -> ReaderAttrs {
    let span = trace_span!("probe_reader_attrs");
    let _enter = span.enter();

    ReaderAttrs {
        vendor_name: card
            .get_attribute(pcsc::Attribute::VendorName, rbuf)
            .map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').into())
            .map_err(|err| {
                warn!("couldn't query reader vendor name: {}", err);
                err
            })
            .ok(),
        friendly_name: card
            .get_attribute(pcsc::Attribute::DeviceFriendlyName, rbuf)
            .map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').into())
            .map_err(|err| {
                warn!("couldn't query reader friendly name: {}", err);
                err
            })
            .ok(),
        system_name: card
            .get_attribute(pcsc::Attribute::DeviceSystemName, rbuf)
            .map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').into())
            .map_err(|err| {
                warn!("couldn't query reader system name: {}", err);
                err
            })
            .ok(),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct EMV {
    pub directory: Option<emv::Directory>,
}

impl EMV {
    fn probe<'a>(card: &mut Card, wbuf: &mut [u8], rbuf: &mut [u8]) -> Option<Self> {
        let span = trace_span!("EMV");
        let _enter = span.enter();

        let mut slf = Self::default();
        match emv::Directory::select(card, wbuf, rbuf) {
            Ok(dir) => {
                debug!("Got an EMV Directory!");
                slf.directory = Some(dir);
            }
            Err(err) => warn!("Couldn't select EMV payment directory: {}", err),
        }

        if slf != EMV::default() {
            Some(slf)
        } else {
            None
        }
    }
}
