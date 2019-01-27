use crate::card::commands::GetResponse;
use crate::core::apdu::{Request, Response, Status};
use crate::core::command::Request as _;
use crate::errors::{ErrorKind, Result};
use log::debug;

pub trait Transport {
    // Performs a raw APDU. As a user, you probably want call_apdu(), not this.
    fn call_raw_apdu(&self, req: &Request) -> Result<Response>;

    // Performs an APDU request, returns the response. Handles extended response bodies and
    // retry-with-Le behaviour transparently as described by the spec, but this isn't actually
    // consistent between different transports/protocols, so you may need to override this if
    // your transport has some oddball behaviour here.
    fn call_apdu(&self, req: Request) -> Result<Response> {
        let res = self.call_raw_apdu(&req)?;
        match res.status {
            Status::OK => Ok(res),
            Status::BytesRemaining(le) => {
                // T=0: If Le is wrong, issue a GET RESPONSE to get the full thing.
                debug!("== RESP: GET RESPONSE with CLA={} Le={:}", req.cla, le);
                self.call_apdu(GetResponse::<()>::new(req.cla, le).to_apdu()?)
            }
            Status::ErrRetryWithLe(le) => {
                // T=1: If Le is wrong, retry it with the correct one.
                debug!("== RETR: Retrying with Le={:}", le);
                self.call_apdu(req.expect(le as usize))
            }
            _ => Err(ErrorKind::StatusError(res.status).into()),
        }
    }
}

impl Transport for () {
    fn call_raw_apdu(&self, _req: &Request) -> Result<Response> {
        Err("() is not a valid transport!".into())
    }
}
