use crate::card::commands::Select;
use crate::card::Card;
use crate::core::command::{Request, Response};
use crate::core::file::FileID;
use crate::errors::Result;
use crate::transport::transport::Transport;

// Interfaces wrap Cards and provide higher-level, application-specific APIs.
pub trait Interface<'a> {
    // Instantiates an interface on an underlying card.
    fn with(card: &'a Card) -> Self;

    // Returns the underlying card.
    fn card(&self) -> &'a Card;

    // Convenience function to execute a higher-order command.
    fn call<ReqT: Request>(&'a self, cmd: &ReqT) -> Result<ReqT::Returns> {
        ReqT::Returns::from_apdu(self.card().call_apdu(cmd.to_apdu()?)?)
    }

    // Execute a SELECT command.
    // TODO: Iterator form of this.
    fn select<'f, T: Interface<'a>>(&'a self, file: &'f FileID) -> Result<T> {
        if let Err(err) = self.call(&Select::new(&file)) {
            return Err(err);
        }
        Ok(T::with(self.card()))
    }
}

impl Interface<'_> for () {
    fn with(_: &'_ Card) -> Self {
        ()
    }

    fn card(&self) -> &'static Card {
        &Card { transport: &() }
    }
}
