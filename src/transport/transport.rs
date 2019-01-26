use crate::core::apdu;
use crate::errors::Result;

pub trait Transport {
    // Performs a raw APDU. As a user, you probably want call_apdu(), not this.
    fn call_raw_apdu(&self, req: &apdu::Request) -> Result<apdu::Response>;

    // Performs an APDU request, returns the response. Handles extended response bodies and
    // retry-with-Le behaviour transparently as described by the spec, but this isn't actually
    // consistent between different transports/protocols, so you may need to override this if
    // your transport has some oddball behaviour here.
    fn call_apdu(&self, req: apdu::Request) -> Result<apdu::Response> {
        let res = self.call_raw_apdu(&req)?;
        match res.status.class() {
            apdu::StatusClass::ErrRetryWithLe(le) => self.call_apdu(req.expect(le as usize)),
            _ => Ok(res),
        }
    }
}
