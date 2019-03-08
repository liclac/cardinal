pub mod get_response;
pub mod read_record;
pub mod select;

use crate::apdu;
use crate::errors::Result;

pub trait Request {
    type Returns: Response;

    // Returns the class byte; this is typically always 0x00 unless your command
    // requires specific bits to be set, as context-sensitive bits will be set on
    // top of it by the transport.
    fn cla(&self) -> u8 {
        0x00
    }

    // Returns the instruction byte.
    fn ins(&self) -> u8;

    // Returns the P1, P2 and command data, if any.
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (0x00, 0x00, Vec::new())
    }

    fn le(&self) -> Option<usize> {
        None
    }

    fn to_apdu(&self) -> Result<apdu::Request> {
        let (p1, p2, data) = self.data();
        let mut req = apdu::Request::new(self.cla(), self.ins(), p1, p2, data);
        req.le = self.le();
        Ok(req)
    }
}

pub trait Response: Sized {
    fn from_apdu(res: apdu::Response) -> Result<Self>;
}

impl Response for () {
    fn from_apdu(_res: apdu::Response) -> Result<Self> {
        Ok(())
    }
}
