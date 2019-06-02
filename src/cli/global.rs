use crate::cli::{Command, Editor, Scope};
use cardinal::errors::{ErrorKind, Result};

// The global scope. This is probably what you want for a top-level scope.
pub struct Global {
    help: HelpCommand,
    exit: ExitCommand,
}

impl Global {
    pub fn new() -> Self {
        Self {
            help: HelpCommand::new(),
            exit: ExitCommand::new(),
        }
    }
}

impl<'a> Scope<'a> for Global {
    fn ps1(&self) -> Vec<String> {
        vec!["~".into()]
    }

    fn commands(&'a self) -> Vec<&'a Command> {
        vec![&self.help, &self.exit]
    }
}

pub struct ExitCommand {}

impl ExitCommand {
    pub fn new() -> Self {
        Self {}
    }
}

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
    fn exec<'a>(
        &self,
        _scope: &'a Scope<'a>,
        _ed: &mut Editor,
        _args: docopt::ArgvMap,
    ) -> Result<()> {
        Err(ErrorKind::CLIExit.into())
    }
}

pub struct HelpCommand {}

impl HelpCommand {
    pub fn new() -> Self {
        Self {}
    }
}

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
    fn exec<'a>(
        &self,
        scope: &'a Scope<'a>,
        _ed: &mut Editor,
        _args: docopt::ArgvMap,
    ) -> Result<()> {
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
