#![allow(deprecated)]

use crate::core::apdu::Status;

error_chain! {
    foreign_links {
        IO(std::io::Error);
        PCSC(pcsc::Error);
    }

    errors {
        StatusError(s: Status) {
            description("apdu response status error")
            display("{:}", s)
        }
    }
}
