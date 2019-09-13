use crate::errors::Result;
use crate::{dump, find_card, Opt};
use cardinal::{emv, Card};
use serde::Serialize;
use structopt::StructOpt;
use tracing::{debug, info, span, Level};

#[derive(Default, Debug, Serialize)]
pub struct Dump {
    pub environment: emv::EnvironmentData,
    pub directory: Vec<emv::DirectoryRecord>,
}

impl Dump {
    pub fn collect<C: Card>(card: &C) -> Result<Self> {
        let pse = emv::Environment::new(card).select()?;
        let psd: Vec<emv::DirectoryRecord> =
            pse.dir_records().collect::<cardinal::errors::Result<_>>()?;
        Ok(Self {
            environment: pse.data,
            directory: psd,
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
    let pse = emv::Environment::new(&card).select()?;
    info!("{:#02x?}", pse);

    debug!("READ RECORD ...");
    for recr in pse.dir_records() {
        let rec = recr?;
        info!("{:#02x?}", rec);
        for entry in rec.record.entries {
            println!(
                "{:16}    {:}    {:}",
                entry
                    .adf_name
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(""),
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
