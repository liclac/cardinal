use crate::card::Card;
use crate::cmd::read_record::{ReadRecord, Record};
use crate::cmd::select::Select;
use crate::cmd::{Request, Response};
use crate::errors::Result;
use crate::file::FileID;
use crate::transport::Transport;

// Interfaces wrap Cards and provide higher-level, application-specific APIs.
pub trait Interface<'a> {
    type SelectResponse: Response;

    // Instantiates an interface on an underlying card.
    fn with(card: &'a Card<'a>, selection: Self::SelectResponse) -> Self;

    // Returns the underlying card.
    fn card(&self) -> &'a Card;

    // Convenience function to execute a higher-order command.
    fn call<ReqT: Request>(&'a self, cmd: &ReqT) -> Result<ReqT::Returns> {
        ReqT::Returns::from_apdu(self.card().call_apdu(cmd.to_apdu()?)?)
    }

    // Execute a SELECT command.
    // TODO: Iterator form of this.
    fn select<'f, T: Interface<'a>>(&'a self, file: &'f FileID) -> Result<T> {
        Ok(T::with(self.card(), self.call(&Select::new(&file))?))
    }

    fn read_record<T: Response>(&'a self, rec: Record) -> Result<T> {
        self.call(&ReadRecord::<T>::new(rec))
    }
}

impl Interface<'_> for () {
    type SelectResponse = ();

    fn with(_: &'_ Card, _: Self::SelectResponse) -> Self {
        ()
    }

    fn card(&self) -> &'static Card {
        &Card { transport: &() }
    }
}
