use crate::card::card::Card;
use crate::core::interface::Interface;

pub struct File<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> Interface<'a> for File<'a> {
    fn with(card: &'a Card) -> Self {
        Self { card: card }
    }
}
