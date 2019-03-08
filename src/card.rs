pub mod commands;

use crate::apdu::{Request, Response};
use crate::errors::Result;
use crate::interface::Interface;
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
}

impl<'a> Interface<'a> for Card<'a> {
    type SelectResponse = ();

    fn with(card: &'a Card, _selection: Self::SelectResponse) -> Self {
        Card {
            transport: card.transport,
        }
    }

    fn card(&self) -> &'a Card {
        self
    }
}

impl<'a> Transport for Card<'a> {
    fn call_raw_apdu(&self, req: &Request) -> Result<Response> {
        self.transport.call_raw_apdu(req)
    }
}
