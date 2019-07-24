use std::io;
use std::io::prelude::*;

/// Wrapper around Writer::write_all(), which still returns the number of bytes written.
pub fn write_all<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<usize> {
    w.write_all(buf)?;
    Ok(buf.len())
}
