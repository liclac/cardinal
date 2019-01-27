use cardinal::adapters::file::File;
use cardinal::card::{Card, Interface};
use cardinal::core::FileID;
use cardinal::errors::Result;
use cardinal::transport::PCSC;
use error_chain::quick_main;
use log::{debug, info};

quick_main!(run);

fn run() -> Result<()> {
    // Init logging...
    simplelog::TermLogger::init(simplelog::LevelFilter::Debug, simplelog::Config::default())
        .unwrap();

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

    // List applications on the card!
    let transport = PCSC::new(scard);
    let card = Card::new(&transport);
    let dir_id = FileID::EF(vec![0x2F, 0x00]);
    let _dir: File = card.select(&dir_id)?;

    Ok(())
}
