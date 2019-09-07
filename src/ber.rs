use error_chain::error_chain;
use nom::number::complete::{be_u16, be_u24, be_u32, be_u8};
use nom::{error::ParseError, multi::length_data, pair, take};
use std::collections::HashMap;

pub type IResult<'a, T> = nom::IResult<&'a [u8], T, Error>;

error_chain! {
    errors {
        Incomplete(needed: nom::Needed) {
            display("{:?}", needed)
        }
        Nom(e: nom::error::ErrorKind) {
            display("{:?}", e)
        }
    }
}

impl nom::error::ParseError<&[u8]> for Error {
    fn from_error_kind(_input: &[u8], kind: nom::error::ErrorKind) -> Self {
        ErrorKind::Nom(kind).into()
    }

    fn append(input: &[u8], kind: nom::error::ErrorKind, other: Self) -> Self {
        other.chain_err(|| Self::from_error_kind(input, kind))
    }
}

impl<'a> From<(&'a [u8], nom::error::ErrorKind)> for Error {
    fn from(e: (&'a [u8], nom::error::ErrorKind)) -> Self {
        Self::from_error_kind(e.0, e.1)
    }
}

impl From<nom::Err<Error>> for Error {
    fn from(e: nom::Err<Error>) -> Self {
        match e {
            nom::Err::Error(err) => err,
            nom::Err::Failure(err) => err,
            nom::Err::Incomplete(needed) => ErrorKind::Incomplete(needed).into(),
        }
    }
}

/// Parses a partial, big-endian u32. If the input is shorter than 4 bytes, it's padded.
fn part_u32(raw: &[u8]) -> IResult<u32> {
    match raw.len() {
        0 => Ok((raw, 0)),
        1 => be_u8(raw).map(|(i, v)| (i, v as u32)),
        2 => be_u16(raw).map(|(i, v)| (i, v as u32)),
        3 => be_u24(raw).map(|(i, v)| (i, v as u32)),
        4 => be_u32(raw),
        _ => Err(nom::Err::Error(
            (raw, nom::error::ErrorKind::TooLarge).into(),
        )),
    }
}

/// Parses a raw TLV tag as a byte sequence. BER-TLV tags can be any length, if the lower 5 bits of the first
/// byte are all set, the next byte is read, and subsequent bytes may set their highest bit to keep going.
pub fn parse_raw_tag(input: &[u8]) -> IResult<&[u8]> {
    if input.len() == 0 {
        return Err(nom::Err::Error((input, nom::error::ErrorKind::Eof).into()));
    }
    for (i, v) in input.iter().enumerate() {
        let more_mask = if i == 0 { 0x1F } else { 0x80 };
        if *v & more_mask != more_mask {
            let (tag, rest) = input.split_at(i + 1);
            return IResult::Ok((rest, tag));
        }
    }
    Err(nom::Err::Incomplete(nom::Needed::Unknown))
}

/// Parses a TLV tag. BER-TLV tags can have any length, but I've never seen one longer than a u32. If you do
/// encounter one, please do widen this. (Or if you prefer, use parse_raw_tag() to handle arbitrary lengths.)
pub fn parse_tag(input: &[u8]) -> IResult<u32> {
    parse_raw_tag(input).and_then(|(i, raw)| IResult::Ok((i, part_u32(raw)?.1)))
}

/// Parses a TLV value's length. If bit 8 of the first byte is 0, bits 1-7 encode the length of the data.
/// If bit 8 is set, bits 1-7 encode the number of subsequent bytes representing the length of the data.
pub fn parse_len(input: &[u8]) -> IResult<usize> {
    let (first, input) = input
        .split_first()
        .ok_or(nom::Err::Incomplete(nom::Needed::Size(1)))?;
    if *first < 128 {
        return IResult::Ok((input, *first as usize));
    }
    take!(input, first - 128)
        .and_then(|(i, v)| IResult::Ok((i, part_u32(v)?.1)))
        .map(|(i, v)| (i, v as usize))
}

/// Parses a raw tag-value pair.
pub fn parse_next_raw(input: &[u8]) -> IResult<(&[u8], &[u8])> {
    pair!(input, parse_raw_tag, length_data(parse_len))
}

/// Parses a tag-value pair.
pub fn parse_next(input: &[u8]) -> IResult<(u32, &[u8])> {
    pair!(input, parse_tag, length_data(parse_len))
}

pub struct TLVIterator<'a> {
    pub input: &'a [u8],
}

pub fn iter<'a>(input: &'a [u8]) -> TLVIterator<'a> {
    TLVIterator::new(input)
}

impl<'a> TLVIterator<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input }
    }
}

impl<'a> TLVIterator<'a> {
    pub fn to_map<V: From<&'a [u8]>>(self) -> Result<HashMap<u32, V>> {
        let mut map = HashMap::new();
        for tvr in self {
            let (tag, value) = tvr?;
            map.insert(tag, value.into());
        }
        Ok(map)
    }
}

impl<'a> Iterator for TLVIterator<'a> {
    type Item = Result<(u32, &'a [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        match parse_next(self.input) {
            Ok((i, v)) => {
                self.input = i;
                Some(Ok(v))
            }
            Err(nom::Err::Error(Error(ErrorKind::Nom(nom::error::ErrorKind::Eof), _))) => None,
            Err(e) => Some(Err(e.into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_part_u32() {
        assert_eq!((&[][..], 0x12), part_u32(&[0x12][..]).unwrap());
        assert_eq!((&[][..], 0x1234), part_u32(&[0x12, 0x34][..]).unwrap());
        assert_eq!(
            (&[][..], 0x123456),
            part_u32(&[0x12, 0x34, 0x56][..]).unwrap(),
        );
        assert_eq!(
            (&[][..], 0x12345678),
            part_u32(&[0x12, 0x34, 0x56, 0x78][..]).unwrap(),
        );
    }

    //#[test]
    //fn test_part_u32_too_long() {
    //    assert_eq!(
    //        IResult::Err(Err::Error((
    //            &[0x12, 0x34, 0x56, 0x78, 0x90][..],
    //            ErrorKind::TooLarge
    //        ))),
    //        part_u32(&[0x12, 0x34, 0x56, 0x78, 0x90][..])
    //    );
    //}

    #[test]
    fn test_parse_raw_tag() {
        assert_eq!(
            (&[0x02, 0x03, 0x04][..], &[0x4F][..]),
            parse_raw_tag(&[0x4F, 0x02, 0x03, 0x04][..]).unwrap(),
        );
        assert_eq!(
            (&[0x02, 0x03, 0x04][..], &[0x5F, 0x50][..]),
            parse_raw_tag(&[0x5F, 0x50, 0x02, 0x03, 0x04][..]).unwrap(),
        );
    }

    #[test]
    fn test_parse_len() {
        assert_eq!((&[][..], 2), parse_len(&[0x02][..]).unwrap());
        assert_eq!((&[][..], 255), parse_len(&[0x81, 0xFF][..]).unwrap());
    }

    #[test]
    fn test_parse_next_raw() {
        assert_eq!(
            (
                &[0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..],
                (&[0x4F][..], &[0x03, 0x04][..])
            ),
            parse_next_raw(&[0x4F, 0x02, 0x03, 0x04, 0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..])
                .unwrap(),
        );
        assert_eq!(
            (&[][..], (&[0x5F, 0x50][..], &[0x04, 0x05, 0x06][..])),
            parse_next_raw(&[0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..]).unwrap(),
        );
    }

    #[test]
    fn test_parse_next() {
        assert_eq!(
            (
                &[0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..],
                (0x4F, &[0x03, 0x04][..])
            ),
            parse_next(&[0x4F, 0x02, 0x03, 0x04, 0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..])
                .unwrap(),
        );
        assert_eq!(
            (&[][..], (0x5F50, &[0x04, 0x05, 0x06][..])),
            parse_next(&[0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..]).unwrap(),
        );
    }

    #[test]
    fn test_iter() {
        assert_eq!(
            vec![(0x4F, &[0x03, 0x04][..]), (0x5F50, &[0x04, 0x05, 0x06][..])],
            iter(&[0x4F, 0x02, 0x03, 0x04, 0x5F, 0x50, 0x81, 0x03, 0x04, 0x05, 0x06][..])
                .collect::<Result<Vec<_>>>()
                .unwrap(),
        );
    }
}
