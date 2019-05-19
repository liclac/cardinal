#![allow(deprecated)]

use crate::apdu::Status;
use error_chain::error_chain;
use serde_json;

error_chain! {
    foreign_links {
        StringFromUtf8(std::string::FromUtf8Error);
        LogSetLoggerError(log::SetLoggerError);
        Docopt(docopt::Error);
        Readline(rustyline::error::ReadlineError);
        JSON(serde_json::Error);
        IO(std::io::Error);
        PCSC(pcsc::Error);
    }

    errors {
        CommandNotFound(name: String) {
            description("command not found")
            display("command not found: {:}", name)
        }
        StatusError(s: Status) {
            description("apdu response status error")
            display("{:}", s)
        }
    }
}
