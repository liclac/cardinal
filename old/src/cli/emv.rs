use crate::cli::{run, Command, Editor, Scope};
use cardinal::app::emv;
use cardinal::card::Card;
use cardinal::errors::Result;
use cardinal::hexjson::HexFormatter;
use log::{info, warn};
use serde::Serialize;
use serde_json::ser::{Formatter, PrettyFormatter};
use std::fmt::Debug;

pub struct EMVCommand<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> EMVCommand<'a> {
    pub fn new(card: &'a Card<'a>) -> Self {
        Self { card }
    }
}

impl<'a> Command for EMVCommand<'a> {
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

    fn exec(&self, scope: &Scope, ed: &mut Editor, _opts: docopt::ArgvMap) -> Result<()> {
        run(ed, &EMVScope::new(scope, self.card))
    }
}

pub struct EMVScope<'a> {
    parent: &'a Scope,
    dump: DumpCommand<'a>,
}

impl<'a> EMVScope<'a> {
    pub fn new(parent: &'a Scope, card: &'a Card<'a>) -> Self {
        Self {
            parent,
            dump: DumpCommand::new(card),
        }
    }
}

impl<'a> Scope for EMVScope<'a> {
    fn ps1(&self) -> Vec<String> {
        let mut ps1 = self.parent.ps1();
        ps1.push("emv".into());
        ps1
    }

    fn commands(&self) -> Vec<&Command> {
        let mut cmds = vec![&self.dump as &Command];
        cmds.append(&mut self.parent.commands());
        cmds
    }
}

pub struct DumpCommand<'a> {
    pub card: &'a Card<'a>,
}

impl<'a> DumpCommand<'a> {
    pub fn new(card: &'a Card<'a>) -> Self {
        Self { card }
    }
}

impl<'a> Command for DumpCommand<'a> {
    fn name(&self) -> &str {
        "dump"
    }

    fn usage(&self) -> &str {
        "dump emv data

Usage:
  dump [--help]

Options:
  --help    Show this message and exit."
    }

    fn exec(&self, _scope: &Scope, _ed: &mut Editor, _opts: docopt::ArgvMap) -> Result<()> {
        // Select the EMV Directory; TODO: Fallbacks when this isn't supported.
        let emv_dir = emv::Directory::select(self.card)?;
        info!("{:}", serialize(&emv_dir.selection)?);

        // Grab and print its records; this explodes if any of them couldn't be read.
        let emv_dir_recs = emv_dir.records().collect::<Result<Vec<_>>>()?;
        for (ie, e) in emv_dir_recs.iter().enumerate() {
            info!("{:}", serialize(&e)?);

            // Each Record contains one or more entries, which can describe one or more
            // applications/files. This makes no sense, but ~sacred legacy behaviour~.
            for (ientry, entry) in e.entries.iter().enumerate() {
                for (iappdef, appdef) in entry.apps.iter().enumerate() {
                    // TODO: Is there a nicer way to warn on nonexistent ADF IDs...?
                    if let Some(id) = &appdef.adf_id {
                        // Select the application! TODO: Query it directly for more data.
                        let emv_app = emv::ADF::select(self.card, id)?;
                        info!("{:}", serialize(&emv_app.selection)?);

                    // debug!("GET PROCESSING OPTIONS");
                    // info!("{:}", serialize(&args, &emv_app.get_processing_options()?)?);
                    } else {
                        warn!(
                            "emv::Directory.records[{:}].entries[{:}].apps[{:}]: has no ADF ID",
                            ie, ientry, iappdef
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

// TODO: Put this somewhere that makes any kind of sense.
fn serialize<T: Serialize + Debug>(v: &T) -> Result<String> {
    // Wrap the built-in pretty-printing JSON formatter in our own formatter,
    // which just formats numbers as hexadecimal instead of decimal.
    to_string_fmt(HexFormatter::new(PrettyFormatter::new()), v)
}

// TODO: Put this somewhere that makes any kind of sense.
fn to_string_fmt<T: Serialize + Debug, F: Formatter>(fmt: F, v: &T) -> Result<String> {
    let mut buf = Vec::with_capacity(128);
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser)?;
    Ok(String::from_utf8(buf)?)
}
