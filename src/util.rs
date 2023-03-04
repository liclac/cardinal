use crate::{Error, Result};
use tracing::{trace, trace_span};

pub(crate) fn call_le<'w, 'r>(
    card: &mut pcsc::Card,
    wbuf: &'w mut [u8],
    rbuf: &'r mut [u8],
    cla: u8,
    ins: u8,
    p1: u8,
    p2: u8,
    le: u16,
) -> Result<&'r [u8]> {
    call_apdu(
        card,
        wbuf,
        rbuf,
        apdu::Command::new_with_le(cla, ins, p1, p2, le),
    )
}

pub(crate) fn call_apdu<'w, 'r>(
    card: &mut pcsc::Card,
    wbuf: &'w mut [u8],
    rbuf: &'r mut [u8],
    cmd: apdu::Command,
) -> Result<&'r [u8]> {
    let span = trace_span!("call_apdu");
    let _enter = span.enter();

    cmd.write(wbuf);
    let req = &wbuf[..cmd.len()];
    trace!(req = format!("{:02X?}", req), ">> TX");

    let rsp = card.transmit(req, rbuf)?;
    let l = rsp.len();
    let (sw1, sw2, data) = (rsp[l - 2], rsp[l - 1], &rsp[..l - 2]);
    trace!(rsp = format!("{:02X?}", rsp), "<< RX");

    if (sw1, sw2) != (0x90, 0x00) {
        Err(Error::APDU(sw1, sw2))
    } else {
        Ok(data)
    }
}

pub(crate) fn expect_tag<'a>(expected: &'a [u8], actual: &'a [u8]) -> Result<&'a [u8]> {
    if expected == actual {
        Ok(expected)
    } else {
        Err(crate::Error::WrongTag {
            expected: expected.into(),
            actual: actual.into(),
        })
    }
}
