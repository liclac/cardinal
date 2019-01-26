use crate::card::Card;

// Interfaces wrap Transports and provide higher-level, application-specific APIs.
pub trait Interface<'a> {
    // Instantiates an interface with a certain transport.
    fn with(card: &'a Card) -> Self;
}
