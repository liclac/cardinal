pub mod global;

use cardinal::errors::{Error, ErrorKind, Result};
use docopt::Docopt;
use log::error;
use rustyline;
use shellwords;

// Wraps an interactive editor. This is technically not specific to cardinal at all.
pub struct Editor {
    ed: rustyline::Editor<()>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            ed: rustyline::Editor::new(),
        }
    }

    // Reads a line of input.
    pub fn readline(&mut self, _scope: &Scope) -> Result<String> {
        Ok(self.ed.readline("~> ")?)
    }
    // Reads a line of input into an Invocation.
    pub fn read<'a>(&mut self, scope: &'a Scope) -> Result<Invocation<'a>> {
        Invocation::parse(scope, self.readline(scope)?.as_str())
    }
    // Reads a line of input and evaluates it.
    pub fn interact<'a>(&mut self, scope: &'a Scope) -> Result<Option<&'a Scope>> {
        self.read(scope)?.exec()
    }

    // Runs a full REPL with a default global state.
    pub fn run_default(&mut self) -> Result<()> {
        self.run(&global::Global::new())
    }
    // Runs a full REPL.
    pub fn run(&mut self, global: &Scope) -> Result<()> {
        let mut scope: Option<&Scope> = Some(global);
        while let Some(s) = scope {
            scope = match self.interact(s) {
                Ok(s) => s,
                Err(Error(ErrorKind::Readline(_), _)) => None,
                Err(e) => {
                    error!("{:}", e);
                    scope
                }
            };
        }
        Ok(())
    }
}

// Wraps a full command invocation. This is basically a fancy Fn, and should be treated as opaque.
pub struct Invocation<'a> {
    scope: &'a Scope,
    cmd: Option<&'a Command>,
    argv: Vec<String>,
}

impl<'a> Invocation<'a> {
    // Parses an input string into an Invocation. Empty input results in a no-op invocation.
    pub fn parse(scope: &'a Scope, input: &str) -> Result<Self> {
        let argv = Self::split(input.trim())?;
        let cmd = argv
            .first()
            .map(|name| {
                scope
                    .iter()
                    .find_map(|s| s.lookup(name))
                    .ok_or_else(|| ErrorKind::CommandNotFound(name.to_string()))
            })
            .transpose()?;
        Ok(Self { scope, cmd, argv })
    }

    pub fn exec(&self) -> Result<Option<&'a Scope>> {
        Ok(match self.cmd {
            Some(cmd) => cmd.call(self.scope, &self.argv)?,
            None => Some(self.scope),
        })
    }

    // Awkward mangling of shellwords' non-fmt::Display'able errors.
    fn split(s: &str) -> Result<Vec<String>> {
        match shellwords::split(s) {
            Ok(words) => Ok(words),
            Err(shellwords::MismatchedQuotes) => Err("unterminated quotes".into()),
        }
    }
}

pub trait Command {
    // Returns a name for the command.
    fn name(&self) -> &str;
    // Usage, in docopt format.
    fn usage(&self) -> &str;
    // Executes the command!
    fn exec<'a>(&self, scope: &'a Scope, opts: &docopt::ArgvMap) -> Result<Option<&'a Scope>>;

    // Executes the command with a list of commandline arguments (where argv[0] is the command name).
    fn call<'a>(&self, scope: &'a Scope, argv: &Vec<String>) -> Result<Option<&'a Scope>> {
        self.exec(
            scope,
            &(Docopt::new(self.usage())?.help(true).argv(argv).parse()?),
        )
    }
}

impl Command for () {
    fn name(&self) -> &str {
        ""
    }
    fn usage(&self) -> &str {
        ""
    }
    fn exec<'a>(&self, scope: &'a Scope, _args: &docopt::ArgvMap) -> Result<Option<&'a Scope>> {
        Ok(Some(scope))
    }
}

pub trait Scope {
    // Returns the parent scope, if any.
    fn parent(&self) -> Option<&Scope>;
    // Returns an iterator over self and the chain of parents.
    fn iter(&self) -> ScopeIterator;
    // Returns an iterator over this scope's "own" commands.
    fn commands(&self) -> Vec<&Command>;

    // Returns the command with the given name.
    fn lookup(&self, name: &str) -> Option<&Command> {
        self.commands()
            .iter()
            .find(|c| c.name() == name)
            .map(|c| *c)
    }
}

pub struct ScopeIterator<'a> {
    scope: Option<&'a Scope>,
}

impl<'a> ScopeIterator<'a> {
    pub fn new(scope: &'a Scope) -> Self {
        Self { scope: Some(scope) }
    }
}

impl<'a> Iterator for ScopeIterator<'a> {
    type Item = &'a Scope;

    fn next(&mut self) -> Option<Self::Item> {
        let scope = self.scope;
        self.scope = scope.and_then(|s| s.parent());
        scope
    }
}
