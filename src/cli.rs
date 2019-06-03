pub mod card;
pub mod emv;
pub mod global;

use cardinal::errors::{Error, ErrorKind, Result};
use docopt::Docopt;
use log::error;
use rustyline;
use shellwords;

/// Wraps an interactive editor. This is technically not specific to cardinal at all.
pub struct Editor {
    ed: rustyline::Editor<()>,
}

impl Editor {
    pub fn new() -> Self {
        Self {
            ed: rustyline::Editor::new(),
        }
    }

    /// Reads a line of input.
    pub fn readline(&mut self, ps1: Vec<String>) -> Result<String> {
        let ps1s: String = ps1.join("> ") + "> ";
        Ok(self.ed.readline(ps1s.as_str())?)
    }
}

pub trait Command {
    /// Returns a name for the command. Used for commandline matching.
    fn name(&self) -> &str;

    /// Usage, in docopt format.
    fn usage(&self) -> &str;

    /// Executes the command!
    fn exec(&self, scope: &Scope, ed: &mut Editor, args: docopt::ArgvMap) -> Result<()>;
}

impl Command for () {
    fn name(&self) -> &str {
        ""
    }
    fn usage(&self) -> &str {
        ""
    }
    fn exec(&self, _s: &Scope, _ed: &mut Editor, _a: docopt::ArgvMap) -> Result<()> {
        Ok(())
    }
}

pub trait Scope {
    /// Returns the PS1 components for this scope, usually appended to the parent scope's.
    fn ps1(&self) -> Vec<String>;

    /// Returns the commands in this scope, usually prepended to the parent scope's.
    /// If two commands exist with the same name, the first one takes precedence.
    fn commands(&self) -> Vec<&Command>;
}

/// Wrapper around shellwords that correctly deals with its nonstandard Errors.
pub fn split(input: &str) -> Result<Vec<String>> {
    match shellwords::split(input) {
        Ok(out) => Ok(out),
        Err(shellwords::MismatchedQuotes) => Err("unterminated quotes".into()),
    }
}

/// Executes the command with a list of commandline arguments, argv[0] is the command name.
pub fn call<A>(cmd: &Command, scope: &Scope, ed: &mut Editor, argv: A) -> Result<()>
where
    A: IntoIterator,
    A::Item: AsRef<str>,
{
    cmd.exec(
        scope,
        ed,
        Docopt::new(cmd.usage())?.help(true).argv(argv).parse()?,
    )
}

pub fn eval(scope: &Scope, ed: &mut Editor, input: &str) -> Result<()> {
    let words = split(input)?;
    let cmd = words
        .first()
        .map(|name| {
            scope
                .commands()
                .into_iter()
                .find(|c| c.name() == name)
                .ok_or(ErrorKind::CommandNotFound("command not found".into()))
        })
        .transpose()?
        .unwrap_or(&());
    call(cmd, scope, ed, words)
}

/// Runs a single read-eval interaction.
pub fn interact(scope: &Scope, ed: &mut Editor) -> Result<()> {
    let input = ed.readline(scope.ps1())?;
    eval(scope, ed, input.as_str())
}

/// Runs a full CLI session using the specified scope as the global one.
pub fn run<S: Scope>(scope: &S) -> Result<()> {
    loop {
        match interact(scope, &mut Editor::new()) {
            Ok(_) => {}
            Err(Error(ErrorKind::Readline(_), _)) | Err(Error(ErrorKind::CLIExit, _)) => {
                break Ok(());
            }
            Err(e) => error!("{:}", e),
        }
    }
}
