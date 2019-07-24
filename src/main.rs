use error_chain::quick_main;
use pcsc;

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

fn run() -> Result<()> {
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
