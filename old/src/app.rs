pub mod emv;

use crate::card::Card;
use crate::cmd::Response;

// Interfaces wrap Cards and provide higher-level, application-specific APIs.
pub trait App<'a> {
    type SelectResponse: Response;

    // Instantiates an interface on an underlying card.
    fn with(card: &'a Card<'a>, selection: Self::SelectResponse) -> Self;

    // Returns the underlying card.
    fn card(&self) -> &'a Card;
}

impl App<'_> for () {
    type SelectResponse = ();

    fn with(_: &'_ Card, _: Self::SelectResponse) -> Self {
        ()
    }

    fn card(&self) -> &'static Card {
        &Card { transport: &() }
    }
}
