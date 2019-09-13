mod cmd_emv;

use cardinal::pcsc::Card as PCard;
use error_chain::quick_main;
use pcsc;
use serde;
use serde_json;
use std::ffi::CString;
use structopt::StructOpt;
use tracing::{debug, span, trace, Level};

mod errors {
    use error_chain::error_chain;
    error_chain! {
        links {
            Cardinal(cardinal::errors::Error, cardinal::errors::ErrorKind);
        }
        foreign_links {
            PCSC(pcsc::Error);
            JSON(serde_json::Error);
            StrUtf8(std::str::Utf8Error);
        }
    }
}
use errors::Result;

fn cmd_readers(_opt: &Opt) -> Result<()> {
    let (_, readers) = list_cards()?;
    for (i, reader) in readers.iter().enumerate() {
        println!("{:3}  {:}", i, reader.to_string_lossy());
    }
    Ok(())
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "readers")]
    /// List all connected readers.
    Readers,

    #[structopt(name = "emv")]
    /// EMV payment card related commands.
    EMV {
        #[structopt(subcommand)]
        cmd: cmd_emv::Command,
    },
}

impl Command {
    fn exec(&self, opt: &Opt) -> Result<()> {
        match self {
            Self::Readers => cmd_readers(opt),
            Self::EMV { cmd } => cmd.exec(opt),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "cardinal", about = "The Swiss army knife of smartcards")]
pub struct Opt {
    #[structopt(short = "r", long = "reader-num", default_value = "0")]
    /// Zero-indexed reader number, if you have multiple.
    reader_num: usize,

    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    /// Every time you -v, it gets noisier (up to -vvv)
    verbosity: u8,

    #[structopt(short = "j", long = "json")]
    /// Dump output as JSON, rather than structs.
    json: bool,

    #[structopt(subcommand)]
    cmd: Command,
}

/// Prints a value to stdout, as Debug or pretty-printed JSON, depending on the --json flag.
pub fn dump<V: serde::Serialize + std::fmt::Debug>(opt: &Opt, value: &V) -> Result<()> {
    if opt.json {
        serde_json::to_writer_pretty(std::io::stdout().lock(), value)?;
    } else {
        println!("{:#02x?}", value);
    }
    Ok(())
}

pub fn list_cards() -> Result<(pcsc::Context, Vec<CString>)> {
    let span = span!(Level::TRACE, "list_cards");
    let _enter = span.enter();

    debug!("Connecting to PCSC...");
    trace!({ scope = "user" }, "pcsc::Context::establish()");
    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;

    debug!("Listing readers...");
    trace!("pcsc::Context::list_readers_len()");
    let mut reader_buf = Vec::with_capacity(ctx.list_readers_len()?);
    reader_buf.resize(reader_buf.capacity(), 0);
    trace!(
        { buf_len = reader_buf.capacity() },
        "pcsc::Context::list_readers()"
    );
    let readers = ctx.list_readers(&mut reader_buf)?;
    Ok((ctx, readers.map(|s| s.into()).collect()))
}

pub fn find_card(opt: &Opt) -> Result<PCard> {
    let span = span!(Level::TRACE, "find_card");
    let _enter = span.enter();

    let (ctx, readers) = list_cards()?;
    let cname = readers
        .iter()
        .skip(opt.reader_num)
        .next()
        .ok_or(pcsc::Error::ReaderUnavailable)?;
    let name = cname.to_str()?;

    debug!({ name }, "Connecting to reader...");
    trace!(
        { name, sharing_mode=?pcsc::ShareMode::Shared, protocols=?pcsc::Protocols::ANY },
        "pcsc::Context::connect()"
    );
    let card = ctx.connect(cname, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?;
    Ok(PCard::wrap(card)?)
}

fn init_logging(opt: &Opt) -> Result<()> {
    Ok(tracing::subscriber::set_global_default(
        tracing_fmt::FmtSubscriber::builder()
            .with_filter(
                tracing_fmt::filter::EnvFilter::try_new(match opt.verbosity {
                    0 => "warn",
                    1 => "info",
                    2 => "debug",
                    _ => "trace",
                })
                .unwrap(),
            )
            .finish(),
    )
    .expect("couldn't set a global logger"))
}

fn run() -> Result<()> {
    let opt = Opt::from_args();
    init_logging(&opt)?;
    opt.cmd.exec(&opt)
}
quick_main!(run);
