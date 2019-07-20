pub mod apdu;

pub use self::apdu::APDU;

use crate::apdu as raw_apdu;
use crate::errors::Result;

pub trait Protocol {
    fn serialize_req(&self, req: &raw_apdu::Request) -> Result<Vec<u8>>;
    fn deserialize_res(&self, data: &[u8]) -> Result<raw_apdu::Response>;
}
