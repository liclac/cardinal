use crate::cli::{Command, Editor, Scope};
use cardinal::card::Card;
use cardinal::errors::Result;

pub struct EmvCommand<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> EmvCommand<'a> {
    pub fn new(card: &'a Card<'a>) -> Self {
        Self { card }
    }
}

impl<'a> Command for EmvCommand<'a> {
    fn name(&self) -> &str {
        "emv"
    }

    fn usage(&self) -> &str {
        "use an emv applet

Usage:
  emv [--help]

Options:
  --help    Show this message and exit."
    }

    fn exec(&self, _scope: &Scope, _ed: &mut Editor, _opts: docopt::ArgvMap) -> Result<()> {
        Ok(())
    }
}
