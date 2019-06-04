mod cli;

use cardinal::card::Card;
use cardinal::errors::{Error, ErrorKind, Result};
use cardinal::transport::pcsc::{Reader, PCSC};
use cli::card::CardScope;
use cli::global::Global;
use docopt::Docopt;
use error_chain::quick_main;
use log::warn;
use serde::{Deserialize, Serialize};
use std::fs::File;

const USAGE: &'static str = "
cardinal - embr's smartcard toy.

Usage:
    cardinal [options] [<reader>]

Options:
    --decimal           Format bytes as decimal instead of hex.
    --trace FILE        Log trace output to FILE.
    -v --verbose        Enable debug logging.
    --help              Show this message.
";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Args {
    arg_reader: Option<usize>,
    flag_decimal: bool,
    flag_trace: Option<String>,
    flag_verbose: bool,
}

fn init_args<T: IntoIterator<Item = S>, S: AsRef<str>>(argv: T) -> Result<Args> {
    Ok(Docopt::new(USAGE)?.argv(argv).deserialize()?)
}

fn init_logging(args: &Args) -> Result<()> {
    let logcfg = simplelog::Config::default();
    let level = if args.flag_verbose {
        simplelog::LevelFilter::Debug
    } else {
        simplelog::LevelFilter::Info
    };

    // We always want to log to the terminal, we may want to add other loggers.
    let mut loggers: Vec<Box<simplelog::SharedLogger>> =
        vec![simplelog::TermLogger::new(level, logcfg).unwrap()];

    // If a trace file is specified, clobber and log traces to it.
    if let Some(trace_file) = args.flag_trace.as_ref() {
        loggers.push(simplelog::WriteLogger::new(
            simplelog::LevelFilter::Trace,
            logcfg,
            File::create(trace_file)?,
        ));
    }

    simplelog::CombinedLogger::init(loggers)?;
    Ok(())
}

quick_main!(run);

fn find_pcsc(id: Option<usize>) -> Result<(String, PCSC)> {
    let readers = Reader::list()?;
    if let Some(i) = id {
        let reader = readers
            .get(i - 1)
            .ok_or::<Error>("index out of range".into())?;
        return Ok((reader.name.clone(), reader.connect()?));
    }
    for reader in readers.iter() {
        match reader.connect() {
            Ok(t) => return Ok((reader.name.clone(), t)),
            Err(Error(ErrorKind::PCSC(pcsc::Error::NoSmartcard), _)) => {
                warn!("Reader has no card inserted: {:}", reader.name);
            }
            Err(e) => return Err(e),
        }
    }
    Err("no smart cards connected".into())
}

fn run() -> Result<()> {
    let args = init_args(std::env::args())?;
    init_logging(&args)?;

    let (name, transport) = find_pcsc(args.arg_reader)?;
    cli::run(
        &mut cli::Editor::new(),
        &CardScope::new(&Global::new(), name, &Card::new(&transport)),
    )
}
