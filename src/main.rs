use cardinal::errors::Result;
use cardinal::transport::PCSC;
use log::{debug, info};

fn main() -> Result<()> {
    simplelog::TermLogger::init(simplelog::LevelFilter::Debug, simplelog::Config::default())
        .unwrap();

    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;
    let mut buf: Vec<u8> = Vec::new();

    buf.resize(ctx.list_readers_len()?, 0);
    let reader = ctx
        .list_readers(&mut buf)?
        .next()
        .expect("No readers connected");
    debug!("Reader: {:?}", reader);

    let scard = ctx.connect(reader, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?;

    buf.resize(scard.get_attribute_len(pcsc::Attribute::AtrString)?, 0);
    info!(
        "ATR: {:X?}",
        scard.get_attribute(pcsc::Attribute::AtrString, &mut buf)?,
    );
    let _card = PCSC::new(scard);

    Ok(())
}
