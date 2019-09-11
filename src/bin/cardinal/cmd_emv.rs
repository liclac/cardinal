use crate::errors::Result;
use crate::{find_card, Opt};
use cardinal::emv;
use structopt::StructOpt;
use tracing::{debug, info, span, Level};

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
    #[structopt(name = "ls")]
    /// List EMV applications on the card.
    Ls {},
}

impl Command {
    pub fn exec(&self, opt: &Opt) -> Result<()> {
        match self {
            Self::Ls {} => cmd_ls(opt),
        };
        Ok(())
    }
}
