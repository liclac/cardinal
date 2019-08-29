use crate::errors::Result;
use crate::iso7816::Select;
use crate::Card;

#[derive(Debug)]
pub struct Directory<C: Card> {
    pub card: C,
}

impl<C: Card> Directory<C> {
    pub fn new(card: C) -> Self {
        Self { card }
    }

    pub fn select(mut self) -> Result<Self> {
        self.card.call(Select::name("1PAY.SYS.DDF01"))?;
        Ok(self)
    }
}
