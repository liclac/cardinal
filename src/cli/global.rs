use crate::cli::{Command, Editor, Scope};
use cardinal::errors::{ErrorKind, Result};

// The global scope. This is probably what you want for a top-level scope.
#[derive(Default)]
pub struct Global {
    help: HelpCommand,
    exit: ExitCommand,
}

impl Global {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Scope for Global {
    fn ps1(&self) -> Vec<String> {
        vec!["~".into()]
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
        "bye!

Usage: exit [--help]

Options:
  --help    Show this screen.
"
    }
    fn exec<'a>(&'a self, _scope: &Scope, _ed: &mut Editor, _args: docopt::ArgvMap) -> Result<()> {
        Err(ErrorKind::CLIExit.into())
    }
}

#[derive(Default)]
pub struct HelpCommand {}

impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }
    fn usage(&self) -> &str {
        "get help

Usage: help [--help]

Options:
    --help    Show this screen.
"
    }
    fn exec<'a>(&self, scope: &Scope, _ed: &mut Editor, _args: docopt::ArgvMap) -> Result<()> {
        println!("");
        for cmd in scope.commands() {
            println!(
                "   {:} - {:}",
                cmd.name(),
                cmd.usage().lines().next().unwrap_or("")
            );
        }
        println!("");
        Ok(())
    }
}
