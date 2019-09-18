use crate::errors::Result;
use crate::{dump, find_card, Opt};
use cardinal::errors::Result as CResult;
use cardinal::{emv, Card, Command as CommandTrait};
use serde::Serialize;
use structopt::StructOpt;
use tracing::{debug, info, span, Level};

#[derive(Default, Debug, Serialize)]
pub struct Dump {
    pub environment: emv::Environment,
    pub directory: Vec<emv::DirectoryRecord>,
    pub apps: Vec<emv::App>,
}

impl Dump {
    pub fn collect<C: Card>(card: &C) -> Result<Self> {
        // Select the PSE (Payment System Environment), which we can use to list applications.
        let pse = emv::Environment::select().call(card)?;

        // The PSD is supposed to consist of a single record, but as always, don't trust that.
        let psd = pse.dir_records(card).collect::<CResult<Vec<_>>>()?;

        // Select each application in the directory, just ignore any that don't work for w/e reason.
        let mut apps = vec![];
        for rec in psd.iter() {
            for entry in rec.record.entries.iter() {
                // TODO: There's no good reason why we'd have to clone() the AID, it never mutates.
                apps.push(entry.adf_name.clone().select().call(card)?);
            }
        }
        Ok(Self {
            environment: pse,
            directory: psd,
            apps,
        })
    }
}

fn cmd_dump(opt: &Opt) -> Result<()> {
    dump(opt, &Dump::collect(&find_card(opt)?)?)
}

fn cmd_ls(opt: &Opt) -> Result<()> {
    let span = span!(Level::TRACE, "cmd_emv::Command::Ls");
    let _enter = span.enter();
    let card = find_card(opt)?;

    debug!("SELECT 1PAY.SYS.DDF01");
    let pse = emv::Environment::select().call(&card)?;
    info!("{:#02x?}", pse);

    debug!("READ RECORD ...");
    for recr in pse.dir_records(&card) {
        let rec = recr?;
        info!("{:#02x?}", rec);
        for entry in rec.record.entries {
            println!(
                "{:016}    {:}    {:}",
                entry.adf_name,
                entry.app_label,
                entry.app_pref_name.unwrap_or_default()
            );
        }
    }

    Ok(())
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "dump")]
    /// Dump all available EMV data on the card.
    Dump {},

    #[structopt(name = "ls")]
    /// List EMV applications on the card.
    Ls {},
}

impl Command {
    pub fn exec(&self, opt: &Opt) -> Result<()> {
        match self {
            Self::Dump {} => cmd_dump(opt),
            Self::Ls {} => cmd_ls(opt),
        }
    }
}
