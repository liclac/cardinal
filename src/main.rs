use error_chain::quick_main;
use pcsc;
use tracing::{debug, span, Level};

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

fn init_logging() -> Result<()> {
    Ok(tracing::subscriber::set_global_default(
        tracing_fmt::FmtSubscriber::builder()
            .with_filter(tracing_fmt::filter::EnvFilter::try_new("debug").unwrap())
            .finish(),
    )
    .expect("couldn't set a global logger"))
}

fn run() -> Result<()> {
    init_logging()?;

    let span = span!(Level::INFO, "main");
    let _enter = span.enter();

    debug!("Connecting to PCSC...");
    let ctx = pcsc::Context::establish(pcsc::Scope::User)?;
    let mut reader_buf = Vec::with_capacity(ctx.list_readers_len()?);
    reader_buf.resize(reader_buf.capacity(), 0);
    let readers = ctx.list_readers(&mut reader_buf)?;
    for name in readers {
        println!("{}", name.to_str()?);
    }

    Ok(())
}
quick_main!(run);
