#![allow(deprecated)]

use crate::core::apdu::Status;
use error_chain::error_chain;

error_chain! {
    foreign_links {
        StringFromUtf8(std::string::FromUtf8Error);
        LogSetLoggerError(log::SetLoggerError);
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
