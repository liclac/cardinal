use crate::cli::{Command, Scope, ScopeIterator};
use cardinal::errors::Result;

// The global scope. This is probably what you want for a top-level scope.
#[derive(Default)]
pub struct Global {
    exit: ExitCommand,
    help: HelpCommand,
}

impl Global {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Scope for Global {
    fn parent(&self) -> Option<&Scope> {
        None
    }
    fn iter(&self) -> ScopeIterator {
        ScopeIterator::new(self)
    }

    fn commands(&self) -> Vec<&Command> {
        vec![&self.help, &self.exit]
    }
}

#[derive(Default)]
pub struct ExitCommand {}

impl Command for ExitCommand {
    fn name(&self) -> &str {
        "exit"
    }
    fn usage(&self) -> &str {
        "exit - bye!"
    }
    fn exec<'a>(&self, _scope: &'a Scope, _args: &Vec<String>) -> Result<Option<&'a Scope>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct HelpCommand {}

impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    fn usage(&self) -> &str {
        "help - get help"
    }
    fn exec<'a>(&self, scope: &'a Scope, _args: &Vec<String>) -> Result<Option<&'a Scope>> {
        println!("");
        for cmd in scope.iter().flat_map(|s| s.commands()) {
            println!("   {:}", cmd.usage().lines().next().unwrap_or(""));
        }
        println!("");
        Ok(Some(scope))
    }
}
