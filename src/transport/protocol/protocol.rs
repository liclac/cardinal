use crate::apdu;
use crate::errors::Result;

pub trait Protocol {
    fn serialize_req(&self, req: &apdu::Request) -> Result<Vec<u8>>;
    fn deserialize_res(&self, data: &[u8]) -> Result<apdu::Response>;
}
