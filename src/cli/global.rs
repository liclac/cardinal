use crate::cli::{Command, Scope, ScopeIterator};
use cardinal::errors::Result;

// The global scope. This is probably what you want for a top-level scope.
#[derive(Default)]
pub struct Global {
    exit: ExitCommand,
}

impl Global {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'a> Scope for Global {
    fn parent(&self) -> Option<&Scope> {
        None
    }
    fn iter(&self) -> ScopeIterator {
        ScopeIterator::new(self)
    }

    fn lookup(&self, name: &str) -> Option<&Command> {
        match name {
            "exit" | "quit" | "q" | "wq" => Some(&self.exit),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct ExitCommand {}

impl Command for ExitCommand {
    fn usage(&self) -> &str {
        "exit - bye!"
    }
    fn exec<'a>(&self, _scope: &'a Scope, _args: &Vec<String>) -> Result<Option<&'a Scope>> {
        Ok(None)
    }
}
