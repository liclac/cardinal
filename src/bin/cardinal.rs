use anyhow::{anyhow, Result};
use clap::Parser as _;
use pcsc::Context;
use tracing::{debug, trace, trace_span};

#[derive(clap::Parser, Debug)]
struct Args {
    /// Increase log level.
    #[arg(short, long, action=clap::ArgAction::Count)]
    verbose: u8,

    /// Decrease log level.
    #[arg(short, long, action=clap::ArgAction::Count)]
    quiet: u8,

    /// Use a specific reader (from --list-readers).
    #[arg(short, long)]
    reader: Option<String>,

    /// Command.
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Probe connected card.
    Probe,

    /// List connected readers.
    ListReaders,
}

impl Command {
    pub fn run(&self, args: &Args) -> Result<()> {
        match self {
            &Self::Probe => self.probe(&args),
            &Self::ListReaders => self.list_readers(&args),
        }
    }

    fn probe(&self, args: &Args) -> Result<()> {
        let span = trace_span!("list_readers");
        let _enter = span.enter();

        let ctx = Context::establish(pcsc::Scope::User)?;
        let mut card = select_card(&ctx, &args.reader)?;
        debug!("Probing card...");
        println!("{:?}", cardinal::probe::probe(&mut card)?);
        Ok(())
    }

    fn list_readers(&self, _args: &Args) -> Result<()> {
        let span = trace_span!("list_readers");
        let _enter = span.enter();

        let ctx = Context::establish(pcsc::Scope::User)?;
        let mut readers_buf = [0; 2048];
        for name in ctx.list_readers(&mut readers_buf)? {
            println!("{}", name.to_str()?);
        }
        Ok(())
    }
}

fn select_card(ctx: &Context, name_: &Option<String>) -> Result<pcsc::Card> {
    let span = trace_span!("select_card", name_);
    let _enter = span.enter();

    Ok(if let Some(name) = name_ {
        debug!(name, "Connecting to named reader");
        // If the --reader flag is passed, use the reader name verbatim.
        ctx.connect(
            std::ffi::CString::new(name.clone())?.as_c_str(), // this is so scuffed lol
            pcsc::ShareMode::Shared,
            pcsc::Protocols::ANY,
        )?
    } else {
        // If not, use the first available reader.
        let mut readers_buf = [0; 2048];
        debug!("Listing available readers");
        let name = ctx
            .list_readers(&mut readers_buf)?
            .next()
            .ok_or(anyhow!("No supported reader connected"))?;

        debug!(?name, "Connecting to first available reader");
        ctx.connect(name, pcsc::ShareMode::Shared, pcsc::Protocols::ANY)?
    })
}

fn init_logging(args: &Args) {
    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(match 2 + args.verbose - args.quiet {
            0 => tracing::Level::ERROR,
            1 => tracing::Level::WARN,
            2 => tracing::Level::INFO,
            3 => tracing::Level::DEBUG,
            4.. => tracing::Level::TRACE,
        })
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logging(&args);
    trace!(?args, "Starting up");
    args.command.run(&args)
}
