mod cli;

use cardinal::app::emv;
use cardinal::card::Card;
use cardinal::errors::{Error, ErrorKind, Result};
use cardinal::hexjson::HexFormatter;
use cardinal::transport::pcsc::{Reader, PCSC};
use cli::card::CardScope;
use cli::global::Global;
use docopt::Docopt;
use error_chain::quick_main;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::ser::{Formatter, PrettyFormatter};
use std::fmt::Debug;
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
    cli::run(&CardScope::new(
        &Global::new(),
        name,
        &Card::new(&transport),
    ))
}

fn dump_emv(args: &Args, card: &Card) -> Result<()> {
    // Select the EMV Directory; TODO: Fallbacks when this isn't supported.
    let emv_dir = emv::Directory::select(&card)?;
    info!("{:}", serialize(&args, &emv_dir.selection)?);

    // Grab and print its records; this explodes if any of them couldn't be read.
    let emv_dir_recs = emv_dir.records().collect::<Result<Vec<_>>>()?;
    for (ie, e) in emv_dir_recs.iter().enumerate() {
        info!("{:}", serialize(&args, &e)?);

        // Each Record contains one or more entries, which can describe one or more
        // applications/files. This makes no sense, but ~sacred legacy behaviour~.
        for (ientry, entry) in e.entries.iter().enumerate() {
            for (iappdef, appdef) in entry.apps.iter().enumerate() {
                // TODO: Is there a nicer way to warn on nonexistent ADF IDs...?
                if let Some(id) = &appdef.adf_id {
                    // Select the application! TODO: Query it directly for more data.
                    let emv_app = emv::ADF::select(&card, id)?;
                    info!("{:}", serialize(&args, &emv_app.selection)?);

                // debug!("GET PROCESSING OPTIONS");
                // info!("{:}", serialize(&args, &emv_app.get_processing_options()?)?);
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

fn serialize<T: Serialize + Debug>(args: &Args, v: &T) -> Result<String> {
    // Wrap the built-in pretty-printing JSON formatter in our own formatter,
    // which just formats numbers as hexadecimal instead of decimal.
    let fmt = PrettyFormatter::new();
    if args.flag_decimal {
        to_string_fmt(fmt, v)
    } else {
        to_string_fmt(HexFormatter::new(fmt), v)
    }
}

fn to_string_fmt<T: Serialize + Debug, F: Formatter>(fmt: F, v: &T) -> Result<String> {
    let mut buf = Vec::with_capacity(128);
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    v.serialize(&mut ser)?;
    Ok(String::from_utf8(buf)?)
}
