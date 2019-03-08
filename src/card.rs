pub mod get_response;
pub mod read_record;
pub mod select;

use crate::apdu;
use crate::app::App;
use crate::cmd::{Request, Response};
use crate::errors::Result;
use crate::file::FileID;
use crate::transport::Transport;

// Magical trait which implements card-like functionality on a transport. You probably
// want to put this on your transport(s) and most of your adapters, unless the adapter
// represents a state in which performing regular card operations does not make sense.
pub struct Card<'a> {
    pub transport: &'a Transport,
}

impl<'a> Card<'a> {
    pub fn new(transport: &'a Transport) -> Self {
        Self { transport }
    }

    // Convenience function to execute a higher-order command.
    pub fn call<ReqT: Request>(&'a self, cmd: &ReqT) -> Result<ReqT::Returns> {
        ReqT::Returns::from_apdu(self.call_apdu(cmd.to_apdu()?)?)
    }

    // Execute a SELECT command.
    // TODO: Iterator form of this.
    pub fn select<'f, T: App<'a>>(&'a self, file: &'f FileID) -> Result<T> {
        Ok(T::with(self, self.call(&select::Select::new(&file))?))
    }

    pub fn read_record<T: Response>(&'a self, rec: read_record::Record) -> Result<T> {
        self.call(&read_record::ReadRecord::<T>::new(rec))
    }
}

impl<'a> Transport for Card<'a> {
    fn call_raw_apdu(&self, req: &apdu::Request) -> Result<apdu::Response> {
        self.transport.call_raw_apdu(req)
    }
}
