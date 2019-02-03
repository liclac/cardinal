use crate::card::card::Card;
use crate::card::interface::Interface;
use crate::core::file::FileID;

pub struct Directory<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> Directory<'a> {
    pub fn id() -> FileID {
        FileID::Name("1PAY.SYS.DDF01".into())
    }

    pub fn select(card: &'a Card) -> Result<Self> {
        card.select::<Self>(&Self::id())
    }
}

impl<'a> Interface<'a> for Directory<'a> {
    fn with(card: &'a Card) -> Self {
        Self { card: card }
    }

    fn card(&self) -> &'a Card {
        self.card
    }
}
