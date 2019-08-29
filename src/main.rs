use cardinal::emv;
use cardinal::pcsc::Card as PCard;
use cardinal::Card;
use error_chain::quick_main;
use pcsc;
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
            StrUtf8(std::str::Utf8Error);
        }
    }
}
use errors::Result;

#[derive(Debug, StructOpt)]
enum EMVCommand {
    #[structopt(name = "ls")]
    /// List EMV applications on the card.
    Ls {},
}

impl EMVCommand {
    fn exec<C: Card>(&self, card: C) -> Result<()> {
        match self {
            Self::Ls {} => {
                emv::Directory::new(card).select()?;
            }
        };
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "emv")]
    /// EMV payment card related commands.
    EMV {
        #[structopt(subcommand)]
        cmd: EMVCommand,
    },
}

impl Command {
    fn exec<C: Card>(&self, card: C) -> Result<()> {
        match self {
            Self::EMV { cmd } => cmd.exec(card),
        }
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "cardinal", about = "The Swiss army knife of smartcards")]
struct Opt {
    #[structopt(short = "r", long = "reader-num", default_value = "0")]
    /// Zero-indexed reader number, if you have multiple.
    reader_num: usize,

    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    /// Every time you -v, it gets noisier (up to -vvv)
    verbosity: u8,

    #[structopt(subcommand)]
    cmd: Command,
}

fn find_card(opt: &Opt) -> Result<PCard> {
    let span = span!(Level::INFO, "find_card");
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

    let cname = readers
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

    let card = find_card(&opt)?;
    opt.cmd.exec(card)
}
quick_main!(run);
