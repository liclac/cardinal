use crate::cli::emv::EmvCommand;
use crate::cli::{run, Command, Editor, Scope};
use cardinal::card::Card;
use cardinal::errors::{Error, Result};
use cardinal::transport::PCSC;
use serde::Deserialize;
use std::ffi::CString;

#[derive(Deserialize)]
struct CardCommandArgs {
    pub arg_num: Option<usize>,
}

#[derive(Default)]
pub struct CardCommand {}

impl CardCommand {
    fn pctx(&self) -> Result<pcsc::Context> {
        Ok(pcsc::Context::establish(pcsc::Scope::User)?)
    }
    fn readers(&self) -> Result<Vec<CString>> {
        let pctx = self.pctx()?;
        let mut buf = Vec::with_capacity(pctx.list_readers_len()?);
        buf.resize(buf.capacity(), 0);
        Ok(pctx
            .list_readers(buf.as_mut_slice())?
            .map(|s| s.into())
            .collect())
    }
}

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

    fn exec(&self, scope: &Scope, _ed: &mut Editor, opts: docopt::ArgvMap) -> Result<()> {
        let opts: CardCommandArgs = opts.deserialize()?;
        let pctx = self.pctx()?;
        let readers = self.readers()?;

        // If this command was called in the form `card n`, pick the n'th (1-indexed) reader and return a CardScope for it.
        if let Some(num) = opts.arg_num {
            let reader_name = readers
                .get(num - 1)
                .ok_or::<Error>("index out of range".into())?;
            return run(&CardScope::new(
                scope,
                String::from(reader_name.to_str()?),
                &Card::new(&PCSC::new(pctx.connect(
                    reader_name,
                    pcsc::ShareMode::Shared,
                    pcsc::Protocols::ANY,
                )?)),
            ));
        }

        println!("Connected readers:");
        println!("");
        for (i, name) in readers.iter().enumerate() {
            println!("{:} - {:}", i + 1, name.to_str()?);
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

    emv: EmvCommand<'a>,
}

impl<'a> CardScope<'a> {
    pub fn new(parent: &'a Scope, name: String, card: &'a Card<'a>) -> Self {
        Self {
            parent: parent,
            card: card,
            name,
            emv: EmvCommand::new(card),
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
