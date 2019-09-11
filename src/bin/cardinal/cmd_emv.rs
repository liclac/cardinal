use crate::errors::Result;
use cardinal::{emv, Card};
use structopt::StructOpt;
use tracing::{debug, info, span, Level};

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "ls")]
    /// List EMV applications on the card.
    Ls {},
}

impl Command {
    pub fn exec<C: Card>(&self, card: &C) -> Result<()> {
        match self {
            Self::Ls {} => {
                let span = span!(Level::TRACE, "cmd_emv::Command::Ls");
                let _enter = span.enter();

                debug!("SELECT 1PAY.SYS.DDF01");
                let pse = emv::Environment::new(card).select()?;
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
            }
        };
        Ok(())
    }
}
