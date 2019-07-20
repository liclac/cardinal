use crate::cli::emv::EMVCommand;
use crate::cli::{run, Command, Editor, Scope};
use cardinal::card::Card;
use cardinal::errors::{Error, Result};
use cardinal::transport::pcsc::Reader;
use serde::Deserialize;

#[derive(Deserialize)]
struct CardCommandArgs {
    pub arg_num: Option<usize>,
}

#[derive(Default)]
pub struct CardCommand {}

impl Command for CardCommand {
    fn name(&self) -> &str {
        "card"
    }
    fn usage(&self) -> &str {
        "select a card or card reader

Usage:
  card [--help]
  card [--help] <num>

Options:
  --help    Show this message and exit."
    }

    fn exec(&self, scope: &Scope, ed: &mut Editor, opts: docopt::ArgvMap) -> Result<()> {
        let opts: CardCommandArgs = opts.deserialize()?;
        let readers = Reader::list()?;

        // If this command was called in the form `card n`, pick the n'th (1-indexed) reader and return a CardScope for it.
        if let Some(num) = opts.arg_num {
            let reader = readers
                .get(num - 1)
                .ok_or::<Error>("index out of range".into())?;
            return run(
                ed,
                &CardScope::new(scope, reader.name.clone(), &Card::new(&reader.connect()?)),
            );
        }

        println!("Connected readers:");
        println!("");
        for (i, reader) in readers.iter().enumerate() {
            println!("{:} - {:}", i + 1, reader.name);
        }
        println!("");
        println!("Use card <num> to activate one.");

        Ok(())
    }
}

pub struct CardScope<'a> {
    pub parent: &'a Scope,
    pub card: &'a Card<'a>,
    pub name: String,

    emv: EMVCommand<'a>,
}

impl<'a> CardScope<'a> {
    pub fn new(parent: &'a Scope, name: String, card: &'a Card<'a>) -> Self {
        Self {
            parent: parent,
            card: card,
            name,
            emv: EMVCommand::new(card),
        }
    }
}

impl<'a> Scope for CardScope<'a> {
    fn ps1(&self) -> Vec<String> {
        vec![self.name.clone()]
    }
    fn commands(&self) -> Vec<&Command> {
        let mut cmds = vec![&self.emv as &Command];
        cmds.extend(self.parent.commands());
        cmds
    }
}
