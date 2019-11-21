mod cmd_emv;

use cardinal::{pcsc, Card, Context};
use error_chain::quick_main;
use serde;
use serde_json;
use std::str::FromStr;
use structopt::StructOpt;
use tracing::{span, Level};

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

fn cmd_readers(opt: &Opt) -> Result<()> {
    let readers = establish_ctx(opt)?.readers()?;
    for (i, reader) in readers.iter().enumerate() {
        println!("{:3}  {:}", i, reader.name());
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

#[derive(Debug, StructOpt, PartialEq, Eq)]
pub enum Interface {
    PCSC,
}

impl FromStr for Interface {
    type Err = errors::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pcsc" => Ok(Self::PCSC),
            _ => Err(format!("unknown interface: {:}", s).into()),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "cardinal", about = "The Swiss army knife of smartcards")]
pub struct Opt {
    #[structopt(short = "r", long = "reader-num", default_value = "0")]
    /// Zero-indexed reader number, if you have multiple.
    reader_num: usize,

    #[structopt(short = "i", long = "interface", default_value = "pcsc")]
    /// Transport interface to use. (pcsc)
    interface: Interface,

    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    /// Every time you -v, it gets noisier (up to -vvv).
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

pub fn establish_ctx(opt: &Opt) -> Result<Box<dyn Context>> {
    Ok(Box::new(match opt.interface {
        Interface::PCSC => pcsc::Context::establish(::pcsc::Scope::User)?,
    }))
}

pub fn find_card(opt: &Opt) -> Result<Box<dyn Card>> {
    let span = span!(Level::TRACE, "find_card");
    let _enter = span.enter();

    let ctx = establish_ctx(&opt)?;
    let readers = ctx.readers()?;
    let reader = readers
        .iter()
        .skip(opt.reader_num)
        .next()
        .ok_or(::pcsc::Error::ReaderUnavailable)?;
    Ok(reader.connect()?)
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
