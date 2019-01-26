use crate::card::commands;
use crate::card::Interface;
use crate::core::command::{Request, Response};
use crate::core::FileID;
use crate::errors::Result;
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
    pub fn call<ReqT: Request>(&self, cmd: &ReqT) -> Result<ReqT::Returns> {
        ReqT::Returns::from_apdu(self.transport.call_apdu(cmd.to_apdu()?)?)
    }

    // Execute a SELECT command.
    // TODO: Iterator form of this.
    pub fn select<T: Interface<'a>>(&'a self, file: &FileID) -> Result<T> {
        if let Err(err) = self.call(&commands::Select::new(&file)) {
            return Err(err);
        }
        Ok(T::with(self))
    }
}
