//! ISO 7816 flavoured BER-TLV (Tag-Length-Value) implementation.
//!
//! BER is an ASN.1 encoding, originally documented in ISO 8825. While a standard ASN.1
//! parser can be used, the subset included in ISO 7816-6 is a slightly odd dialect, and
//! the ecosystem around it has some oddball conventions, most notably referring to tags
//! by their hex values (0x6F rather than Application 15).
//!
//! (There are other ASN.1 crates out there, but this format is so trivial that it pretty
//! quickly became easier to just parse it myself than to try to persuade them to swallow
//! oddball responses from my cards and converting tags between hex and namespaces.)
//!
//! Aside from ISO 7816-6, this dialect is also documented in EMV Book 3, Annex B, which
//! is freely available from EMVCo's website. For ease of access, this implementation is
//! written using the EMV specs rather than ISO 7816 or ISO 8825 unless otherwise noted.

use byteorder::{BigEndian, ByteOrder};
use nom::bytes::complete::take;
use nom::number::complete::be_u8;

pub type IResult<'a, T> = nom::IResult<&'a [u8], T>;

/// Does this tag represent a constructed value?
///
/// A constructed value contains further TLV tuples. The opposite is a primitive value,
/// which is a value in itself (a string, number, etc. depending on context).
pub fn is_constructed(tag: &[u8]) -> bool {
    tag.first().unwrap_or(&0) & (1 << 5) != 0
}

/// Parses a tag.
///
/// If bits 1-5 of the first byte are all set, this is a multi-byte tag, continuing until
/// and including the first subsequent byte without bit 8 set.
///
/// See EMV Book 3, Annex B1: "Coding of the Tag Field of BER-TLV Data Objects".
fn take_tag(data: &[u8]) -> IResult<&[u8]> {
    let (rest, short) = take(1usize)(data)?;
    if short[0] & 0b0001_1111 != 0b0001_1111 {
        Ok((rest, short))
    } else {
        let mut tag_len = 2usize;
        for b in rest {
            if b & (1 << 7) != 0 {
                tag_len += 1;
            } else {
                break;
            }
        }
        take(tag_len)(data)
    }
}

/// Parses a length field.
fn take_len(data_: &[u8]) -> IResult<usize> {
    let (data, lenlen) = be_u8(data_)?;
    if lenlen <= 127 {
        Ok((data, lenlen as usize))
    } else {
        let lensize = (lenlen & 0b0111_1111) as usize;
        // Error out if the length is too large for the target architecture, or if
        // it's indeterminate (0b1000_0000). Indeterminate lengths are technically
        // valid BER according to ISO 8825, but not allowed in ISO 7816 or EMV.
        if lensize < 1 || lensize > 8 {
            Err(nom::Err::Error(nom::error::Error::new(
                data_, // Return the full input!
                nom::error::ErrorKind::TooLarge,
            )))
        } else {
            Ok((
                &data[lensize..],
                BigEndian::read_uint(data, lensize) as usize,
            ))
        }
    }
}

/// Parses the next (tag, value) pair from a BER-TLV blob.
pub fn parse_next(data: &[u8]) -> IResult<(&[u8], &[u8])> {
    let (data, tag) = take_tag(data)?;
    let (data, len) = take_len(data)?;
    let (data, val) = take(len)(data)?;
    Ok((data, (tag, val)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_constructed_0x6f() {
        assert_eq!(is_constructed(&[0x6F]), true); // ISO 7816: FCI Template.
    }
    #[test]
    fn test_is_constructed_0xbf0c() {
        assert_eq!(is_constructed(&[0xBF, 0x0C]), true); // EMV: FCI Issuer Discretionary Data.
    }
    #[test]
    fn test_is_constructed_0x84() {
        assert_eq!(is_constructed(&[0x84]), false); // ISO 7816: FCI Template > DF Name.
    }
    #[test]
    fn test_is_constructed_0x5f2d() {
        assert_eq!(is_constructed(&[0x5F, 0x2D]), false); // EMV: Language Preference.
    }

    #[test]
    fn test_take_tag_0x6f() {
        assert_eq!(
            take_tag(&[0x6F, 0xFF]).expect("couldn't take tag"),
            (&[0xFF][..], &[0x6F][..])
        );
    }
    #[test]
    fn test_take_tag_0xbf0c() {
        assert_eq!(
            take_tag(&[0xBF, 0x0C, 0x00]).expect("couldn't take tag"),
            (&[0x00][..], &[0xBF, 0x0C][..])
        );
    }
    #[test]
    fn test_take_tag_0x5f2d() {
        let (rest, tag) =
            take_tag(&[0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F]).expect("couldn't take tag");
        assert_eq!(tag, &[0x5F, 0x2D]);
        assert_eq!(rest, &[0x02, 0x65, 0x6E, 0x9F]);
    }

    #[test]
    fn test_take_length_short() {
        assert_eq!(
            take_len(&[0b0000_0000, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0)
        );
        assert_eq!(
            take_len(&[0b0000_0001, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 1)
        );
        assert_eq!(
            take_len(&[0b0111_1111, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 127)
        );
    }
    #[test]
    fn test_take_length_u8() {
        assert_eq!(
            take_len(&[0b1000_0001, 0x00, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0001, 0xFF, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0xFF)
        );
    }
    #[test]
    fn test_take_length_u16() {
        assert_eq!(
            take_len(&[0b1000_0010, 0x00, 0x00, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0010, 0x12, 0x34, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x1234)
        );
        assert_eq!(
            take_len(&[0b1000_0010, 0xFF, 0xFF, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0xFFFF)
        );
    }
    #[test]
    fn test_take_length_u24() {
        assert_eq!(
            take_len(&[0b1000_0011, 0x00, 0x00, 0x00, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0011, 0x12, 0x34, 0x56, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x123456)
        );
        assert_eq!(
            take_len(&[0b1000_0011, 0xFF, 0xFF, 0xFF, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u32() {
        assert_eq!(
            take_len(&[0b1000_0100, 0x00, 0x00, 0x00, 0x00, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0100, 0x12, 0x34, 0x56, 0x78, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0x12345678)
        );
        assert_eq!(
            take_len(&[0b1000_0100, 0xFF, 0xFF, 0xFF, 0xFF, 0xED]).expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u40() {
        assert_eq!(
            take_len(&[0b1000_0101, 0x00, 0x00, 0x00, 0x00, 0x00, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0101, 0x12, 0x34, 0x56, 0x78, 0x90, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x1234567890)
        );
        assert_eq!(
            take_len(&[0b1000_0101, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u48() {
        assert_eq!(
            take_len(&[0b1000_0110, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0110, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x123456789012)
        );
        assert_eq!(
            take_len(&[0b1000_0110, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFFFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u56() {
        assert_eq!(
            take_len(&[0b1000_0111, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[0b1000_0111, 0x12, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0x12345678901234)
        );
        assert_eq!(
            take_len(&[0b1000_0111, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xED])
                .expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFFFFFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u64() {
        assert_eq!(
            take_len(&[
                0b1000_1000,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0x00,
                0xED
            ])
            .expect("couldn't take length"),
            (&[0xED][..], 0x00)
        );
        assert_eq!(
            take_len(&[
                0b1000_1000,
                0x12,
                0x34,
                0x56,
                0x78,
                0x90,
                0x12,
                0x34,
                0x56,
                0xED
            ])
            .expect("couldn't take length"),
            (&[0xED][..], 0x1234567890123456)
        );
        assert_eq!(
            take_len(&[
                0b1000_1000,
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                0xFF,
                0xED
            ])
            .expect("couldn't take length"),
            (&[0xED][..], 0xFFFFFFFFFFFFFFFF)
        );
    }
    #[test]
    fn test_take_length_u72() {
        let data = &[
            0b1000_1001,
            0x12,
            0x34,
            0x56,
            0x78,
            0x90,
            0x12,
            0x34,
            0x56,
            0xED,
        ];
        assert_eq!(
            take_len(data).expect_err("taking u72 length didn't fail"),
            nom::Err::Error(nom::error::Error::new(
                &data[..],
                nom::error::ErrorKind::TooLarge
            ))
        );
    }
    #[test]
    fn test_take_length_indeterminate() {
        // Setting the multi-byte flag and specifying zero bytes is technically valid in
        // BER and means "indeterminate length", but not valid in the ISO 7816 subset.
        assert_eq!(
            take_len(&[0b1000_0000, 0xED]).expect_err("taking indeterminate length didn't fail"),
            nom::Err::Error(nom::error::Error::new(
                &[0b1000_0000, 0xED][..],
                nom::error::ErrorKind::TooLarge
            ))
        );
    }

    #[test]
    fn test_parse_tlv_emv_dir() {
        // Response to `SELECT '1PAY.SYS.DDF01'` to a (Nitecrest) Monzo card.
        let (rest, (tag, val)) = parse_next(&[
            0x6F, 0x1E, 0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44,
            0x44, 0x46, 0x30, 0x31, 0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E,
            0x9F, 0x11, 0x01, 0x01,
        ])
        .expect("couldn't parse TLV");
        assert_eq!(tag, &[0x6F]);
        assert_eq!(is_constructed(tag), true);
        assert_eq!(
            val,
            &[
                0x84, 0x0E, 0x31, 0x50, 0x41, 0x59, 0x2E, 0x53, 0x59, 0x53, 0x2E, 0x44, 0x44, 0x46,
                0x30, 0x31, 0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11,
                0x01, 0x01
            ]
        );
        assert_eq!(rest, &[]);

        // Parse 0x6F - the FCI Template.
        let (rest, (tag, val)) = parse_next(val).expect("couldn't parse 0x6F[0]");
        assert_eq!(tag, &[0x84]);
        assert_eq!(is_constructed(tag), false);
        assert_eq!(val, "1PAY.SYS.DDF01".as_bytes());
        assert_eq!(
            rest,
            &[0xA5, 0x0C, 0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01]
        );

        let (rest, (tag, val)) = parse_next(rest).expect("couldn't parse 0x6F[1]");
        assert_eq!(tag, &[0xA5]);
        assert_eq!(is_constructed(tag), true);
        assert_eq!(
            val,
            &[0x88, 0x01, 0x01, 0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01]
        );
        assert_eq!(rest, &[]);

        // Parse 0xA5 - the FCI Proprietary Template.
        let (rest, (tag, val)) = parse_next(val).expect("couldn't parse 0x6F[1] 0xA5[0]");
        assert_eq!(tag, &[0x88]);
        assert_eq!(is_constructed(tag), false);
        assert_eq!(val, &[0x01]);
        assert_eq!(
            rest,
            &[0x5F, 0x2D, 0x02, 0x65, 0x6E, 0x9F, 0x11, 0x01, 0x01]
        );

        let (rest, (tag, val)) = parse_next(rest).expect("couldn't parse 0x6F[1] 0xA5[1]");
        assert_eq!(tag, &[0x5F, 0x2D]);
        assert_eq!(is_constructed(tag), false);
        assert_eq!(val, "en".as_bytes());
        assert_eq!(rest, &[0x9F, 0x11, 0x01, 0x01]);

        let (rest, (tag, val)) = parse_next(rest).expect("couldn't parse 0x6F[1] 0xA5[2]");
        assert_eq!(tag, &[0x9F, 0x11]);
        assert_eq!(is_constructed(tag), false);
        assert_eq!(val, &[0x01]);
        assert_eq!(rest, &[]);
    }
}
