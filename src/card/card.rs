use crate::card::commands;
use crate::core::command::{Request, Response};
use crate::core::FileID;
use crate::core::Interface;
use crate::errors::Result;
use crate::transport::Transport;

// Magical trait which implements card-like functionality on a transport. You probably
// want to put this on your transport(s) and most of your adapters, unless the adapter
// represents a state in which performing regular card operations does not make sense.
pub trait Card: Transport {
    // Convenience function to execute a higher-order command.
    fn call<ReqT: Request>(&self, cmd: &ReqT) -> Result<ReqT::Returns> {
        ReqT::Returns::from_apdu(self.call_apdu(cmd.to_apdu()?)?)
    }

    // Execute a SELECT command.
    // TODO: Iterator form of this.
    fn select<T: Interface>(&self, file: &FileID) -> Result<T> {
        if let Err(err) = self.call(&commands::Select::new(&file)) {
            return Err(err);
        }
        Ok(T::with(self))
    }
}
