use crate::apdu;
use crate::cmd::Request;

pub struct GetProcessingOptions {}

impl GetProcessingOptions {
    pub fn new() -> Self {
        Self {}
    }
}

impl Request for GetProcessingOptions {
    type Returns = apdu::Response;

    fn cla(&self) -> u8 {
        0x80
    }
    fn ins(&self) -> u8 {
        0xA8
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (0x00, 0x00, vec![0x83, 0x00])
    }
}
