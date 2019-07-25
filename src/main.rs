use error_chain::quick_main;
use pcsc;
use structopt::StructOpt;
use tracing::{debug, info, span, trace, Level};

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
#[structopt(name = "cardinal", about = "The Swiss army knife of smartcards")]
struct Opt {
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    /// Every time you -v, it gets noisier (up to -vvv)
    verbosity: u8,
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

    let span = span!(Level::INFO, "main");
    let _enter = span.enter();

    debug!("Connecting to PCSC...");
    trace!("pcsc::Context::establish()");
    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;
    trace!("pcsc::Context::list_readers_len()");
    let mut reader_buf = Vec::with_capacity(ctx.list_readers_len()?);
    reader_buf.resize(reader_buf.capacity(), 0);
    trace!(
        { list_readers_len = reader_buf.capacity() },
        "pcsc::Context::list_readers()"
    );
    let readers = ctx.list_readers(&mut reader_buf)?;
    for name in readers {
        info!({ name = name.to_str()? }, "Found reader!");
    }

    Ok(())
}
quick_main!(run);
