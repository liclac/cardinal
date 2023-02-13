use crate::{Error, Result};
use tracing::{trace, trace_span};

pub(crate) fn call_le<'a>(
    card: &mut pcsc::Card,
    wbuf: &'a mut [u8],
    rbuf: &'a mut [u8],
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    le: u16,
) -> Result<&'a [u8]> {
    call_apdu(
        card,
        wbuf,
        rbuf,
        apdu::Command::new_with_le(cla, ins, p1, p2, le),
    )
}

pub(crate) fn call_apdu<'a>(
    card: &mut pcsc::Card,
    wbuf: &'a mut [u8],
    rbuf: &'a mut [u8],
    cmd: apdu::Command,
) -> Result<&'a [u8]> {
    let span = trace_span!("call_apdu");
    let _enter = span.enter();

    cmd.write(wbuf);
    let req = &wbuf[..cmd.len()];
    trace!(?req, ">> TX");

    let rsp = card.transmit(req, rbuf)?;
    trace!(?rsp, "<< RX");
    let l = rsp.len();
    let (sw1, sw2, data) = (rsp[l - 2], rsp[l - 1], &rsp[..l - 2]);

    if (sw1, sw2) != (0x90, 0x00) {
        Err(Error::APDU(sw1, sw2))
    } else {
        Ok(data)
    }
}
