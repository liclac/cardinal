use crate::core::apdu;
use crate::errors::Result;

pub trait Protocol {
    fn serialize_req(req: &apdu::Request) -> Result<Vec<u8>>;
    fn deserialize_res(data: &[u8]) -> Result<apdu::Response>;
}
