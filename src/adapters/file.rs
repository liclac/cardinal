use crate::card::card::Card;
use crate::card::interface::Interface;

pub struct File<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> Interface<'a> for File<'a> {
    fn with(card: &'a Card) -> Self {
        Self { card: card }
    }

    fn card(&self) -> &'a Card {
        self.card
    }
}
