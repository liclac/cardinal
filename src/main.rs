use cardinal::adapters::emv;
use cardinal::card::{Card, Interface};
use cardinal::core::apdu::{self, Status};
use cardinal::errors::{Error, ErrorKind, Result};
use cardinal::transport::PCSC;
use error_chain::quick_main;
use log::{debug, error, info};
use std::fs::File;

quick_main!(run);

fn init_logging() -> Result<()> {
    let logcfg = simplelog::Config::default();
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(simplelog::LevelFilter::Info, logcfg).unwrap(),
        simplelog::WriteLogger::new(
            simplelog::LevelFilter::Trace,
            logcfg,
            File::create("cardinal_trace.log")?,
        ),
    ])?;
    Ok(())
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

    // List applications on the card!
    let transport = PCSC::new(scard);
    let card = Card::new(&transport);
    let emv_dir = emv::Directory::select(&card)?;
    println!("{:#x?}", emv_dir.selection);
    for i in 0x01..=0xFF as u8 {
        match emv_dir.read_record::<apdu::Response>(emv_dir.record_num(i)?) {
            Ok(r) => info!("{:} => {:#x?}", i, r),
            Err(Error(ErrorKind::StatusError(Status::ErrRecordNotFound), _)) => break,
            Err(e) => error!("{:} => {:?}", i, e),
        };
    }

    Ok(())
}
