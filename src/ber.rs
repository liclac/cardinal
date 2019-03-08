use crate::errors::Result;
use std::collections::HashMap;

// Reads a byte from a buffer, and moves the pointer ahead one byte.
fn nom(buf: &mut &[u8]) -> Result<u8> {
    if let Some((b, rest)) = buf.split_first() {
        *buf = rest;
        Ok(*b)
    } else {
        Err("data truncated".into())
    }
}

// Reads the next tag in a BER TLV sequence.
// The buffer must be pointing to the length field. If this call succeeds, buf will
// point to the length, otherwise buf will point to an undefined location.
pub fn read_tag(buf: &mut &[u8]) -> Result<u32> {
    let mut tag = nom(buf)? as u32;

    if (tag & 0x1F) == 0x1F {
        // Long tag
        loop {
            let b = nom(buf)?;
            tag = (tag << 8) | (b as u32);

            if (b & 0x80) == 0 {
                return Ok(tag);
            }
        }
    } else {
        // Short tag
        return Ok(tag);
    }
}

// Reads the length of the next value in a BER TLV sequence.
// The buffer must be pointing to the length field. If this call succeeds, buf will
// point to the data, otherwise buf will point to an undefined location.
pub fn read_length(buf: &mut &[u8]) -> Result<u32> {
    match nom(buf)? {
        x @ 0x00...0x80 => Ok(x as u32),
        0x81 => Ok(nom(buf)? as u32),
        0x82 => {
            let b0 = nom(buf)? as u32;
            let b1 = nom(buf)? as u32;
            Ok(b0 << 8 | b1)
        }
        // Omitting 83/etc as not necessary for smartcards
        x => Err(format!("invalid length byte: {:}", x).into()),
    }
}

// Reads a tag + data from a BER TLV sequence.
// The buffer must be pointing to the start of a tag. If this call succeeds, buf will
// point to the start of the next tag, or if this is the last tag in the sequence,
// it will be a zero-length slice.
pub fn read_tlv<'a>(buf: &mut &'a [u8]) -> Result<(u32, &'a [u8])> {
    let tag = read_tag(buf)?;
    let len = read_length(buf)? as usize;
    if buf.len() < len as usize {
        return Err(format!(
            "tag truncated: {:}, len={:}, remaining={:}",
            tag,
            len,
            buf.len()
        )
        .into());
    }

    let val = &buf[0..len];
    *buf = &buf[len..];

    Ok((tag, val))
}

pub struct Iter<'a> {
    pub buf: &'a [u8],
}

pub fn iter<'a>(buf: &'a [u8]) -> Iter<'a> {
    Iter::new(buf)
}

impl<'a> Iter<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(u32, &'a [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.len() == 0 {
            None
        } else {
            Some(read_tlv(&mut self.buf))
        }
    }
}

pub type Map = HashMap<u32, Vec<u8>>;

pub fn to_map<'a>(buf: &'a [u8]) -> Result<Map> {
    let mut map = Map::new();
    for tvr in iter(buf) {
        let (tag, value) = tvr?;
        map.insert(tag, value.into());
    }
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_tlv() {
        let b = vec![0x1F, 0x8E, 0x0E, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
        let mut bp = &b[..];

        let (tag, value) = read_tlv(&mut bp).unwrap();
        assert_eq!(0x1f8e0e, tag);
        assert_eq!(value, [0x68, 0x65, 0x6c, 0x6c, 0x6f]);
    }

    #[test]
    fn test_iter() {
        let b = vec![0x1F, 0x8E, 0x0E, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
        let vec: Vec<Result<(u32, &[u8])>> = iter(&b).collect();
        let expect: Vec<Result<(u32, &[u8])>> =
            vec![Ok((0x1f8e0e, &[0x68, 0x65, 0x6c, 0x6c, 0x6f]))];
        assert_eq!(1, vec.len());
        assert_eq!(vec[0].as_ref().unwrap(), expect[0].as_ref().unwrap());
    }

    #[test]
    fn test_select_emv() {
        // SELECT 1PAY.SYS.DDF01 from a Swedish Debit Mastercard (ICA Banken).
        let b = vec![
            0x6f, 0x20, 0x84, 0xe, 0x31, 0x50, 0x41, 0x59, 0x2e, 0x53, 0x59, 0x53, 0x2e, 0x44,
            0x44, 0x46, 0x30, 0x31, 0xa5, 0xe, 0x88, 0x01, 0x01, 0x5f, 0x2d, 0x4, 0x73, 0x76, 0x65,
            0x6e, 0x9f, 0x011, 0x01, 0x01,
        ];
        for (i, tvr) in iter(&b).enumerate() {
            // This should contain only a single value.
            assert_eq!(0, i);
            let (tag, value) = tvr.expect("iterator failed");

            // 0x6F - FCI Template.
            assert_eq!(tag, 0x6f);
            assert_eq!(
                value,
                [
                    0x84, 0xe, 0x31, 0x50, 0x41, 0x59, 0x2e, 0x53, 0x59, 0x53, 0x2e, 0x44, 0x44,
                    0x46, 0x30, 0x31, 0xa5, 0xe, 0x88, 0x1, 0x1, 0x5f, 0x2d, 0x4, 0x73, 0x76, 0x65,
                    0x6e, 0x9f, 0x11, 0x1, 0x1
                ],
            );
            for (i, tvr) in iter(&value).enumerate() {
                let (tag, value) = tvr.expect("FCI Template iterator failed");
                match i {
                    0 => {
                        // 0x84 - DF Name.
                        assert_eq!(tag, 0x84);
                        assert_eq!(value, "1PAY.SYS.DDF01".as_bytes());
                    }
                    1 => {
                        // 0xA5 - FCI Proprietary Template.
                        assert_eq!(tag, 0xa5);
                        assert_eq!(
                            value,
                            [
                                0x88, 0x1, 0x1, 0x5f, 0x2d, 0x4, 0x73, 0x76, 0x65, 0x6e, 0x9f,
                                0x11, 0x1, 0x1
                            ],
                        );

                        for (i, tvr) in iter(&value).enumerate() {
                            let (tag, value) = tvr.expect("FCI PT iterator failed");
                            match i {
                                0 => {
                                    // 0x88 - SFI of the Directory Elementary File.
                                    assert_eq!(tag, 0x88);
                                    assert_eq!(value, [0x01]);
                                }
                                1 => {
                                    // 0x5F2D - Language preference.
                                    assert_eq!(tag, 0x5f2d);
                                    assert_eq!(value, "sven".as_bytes());
                                }
                                2 => {
                                    // 0x9F11 - Issuer Code Table Index.
                                    assert_eq!(tag, 0x9F11);
                                    assert_eq!(value, [0x01]);
                                }
                                _ => panic!(
                                    "Unexpected item in FCI PT ({:}): {:#x} => {:#x?}",
                                    i, tag, value
                                ),
                            }
                        }
                    }
                    _ => panic!(
                        "Unexpected item in FCI Template ({:}): {:#x} => {:#x?}",
                        i, tag, value
                    ),
                };
            }
        }
    }
}
