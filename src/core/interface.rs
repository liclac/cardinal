use crate::card::Card;

// Interfaces wrap Transports and provide higher-level, application-specific APIs.
pub trait Interface<'a, CardT: Card> {
    // Instantiates an interface with a certain transport.
    fn with(card: &'a CardT) -> Self;
}
