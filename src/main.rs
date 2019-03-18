use cardinal::app::emv;
use cardinal::card::Card;
use cardinal::errors::Result;
use cardinal::transport::PCSC;
use error_chain::quick_main;
use log::{debug, info, warn};
use std::fmt::Debug;
use std::fs::File;

quick_main!(run);

fn init_logging() -> Result<()> {
    let logcfg = simplelog::Config::default();
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(simplelog::LevelFilter::Debug, logcfg).unwrap(),
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Trace,
            logcfg,
            File::create("cardinal_trace.log")?,
        ),
    ])?;
    Ok(())
}

fn serialize<T: serde::Serialize + Debug>(v: &T) -> Result<String> {
    Ok(format!("{:#X?}", v))
}

fn run() -> Result<()> {
    init_logging()?;

    // Find a card reader, connect to the first one we do.
    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;
    let mut buf: Vec<u8> = Vec::new();

    buf.resize(ctx.list_readers_len()?, 0);
    let reader = ctx
        .list_readers(&mut buf)?
        .next()
        .expect("No readers connected");
    debug!("Reader: {:?}", reader);

    let scard = ctx.connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?;

    // Read the ATR from the card. TODO: Parse this.
    buf.resize(scard.get_attribute_len(pcsc::Attribute::AtrString)?, 0);
    info!(
        "ATR: {:X?}",
        scard.get_attribute(pcsc::Attribute::AtrString, &mut buf)?,
    );

    let transport = PCSC::new(scard);
    let card = Card::new(&transport);

    // Select the EMV Directory; TODO: Fallbacks when this isn't supported.
    let emv_dir = emv::Directory::select(&card)?;
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
                    let emv_app = emv::ADF::select(&card, id)?;
                    info!("{:}", serialize(&emv_app.selection)?);
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
