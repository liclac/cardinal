//! ATR (Answer-to-Reset) parser.
//!
//! Aside from ISO 7816, this is covered by the EMV L1 Contact Interface Specification,
//! Section 8: "Answer to Reset", which is freely available from EMVCo's website.
//! For ease of access, this is written using the EMV specs, as well as the surprisingly
//! complete Wikipedia page: https://en.wikipedia.org/wiki/Answer_to_reset
//!
//! Useful online ATR parser: https://smartcard-atr.apdu.fr/

use nom::bytes::complete::take;
use nom::combinator::{cond, map};
use nom::number::complete::be_u8;
use num_enum::{FromPrimitive, IntoPrimitive};

pub type IResult<'a, T> = nom::IResult<&'a [u8], T>;

/// Initial Character TS, a known bit pattern to tell electrical transmission convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum TS {
    /// Direct Convention, 1 is high - (H)LHHLHHHLLH.
    Direct = 0x3B,

    /// Inverse Convention, 1 is low - (H)LHHLLLLLLH.
    /// This is relatively rare, and EMV (but not ISO 7816) has deprecated this form.
    Inverse = 0x3F,

    /// Invalid TS.
    #[num_enum(catch_all)]
    Invalid(u8) = 0xFF,
}

/// Format Byte indicating which other bytes are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct T0 {
    /// K, aka number of historical bytes present.
    pub k: u8,
    /// Bitmask: Whether the ta1, tb1, tc1 and td1 bytes are present.
    pub tx1: u8,
}

impl From<u8> for T0 {
    fn from(v: u8) -> Self {
        Self {
            k: v & 0b0000_1111,
            tx1: (v & 0b1111_0000) >> 4,
        }
    }
}

/// A transmission protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Protocol {
    T0 = 0,
    T1 = 1,
    #[num_enum(catch_all)]
    Invalid(u8) = 0xFF,
}

/// Interface Byte, describing a protocol and whether further bytes are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TDn {
    /// Protocol, eg. T=0 or T=1.
    pub protocol: Protocol,
    /// Bitmask: Whether the next t(x)(n+1) bytes are present.
    pub txn: u8,
}

impl From<u8> for TDn {
    fn from(v: u8) -> Self {
        Self {
            protocol: (v & 0b0000_1111).into(),
            txn: (v & 0b1111_0000) >> 4,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TXn<Ta: From<u8>, Tb: From<u8>, Tc: From<u8>> {
    pub ta: Option<Ta>,
    pub tb: Option<Tb>,
    pub tc: Option<Tc>,
    pub td: Option<TDn>,
}

fn parse_txn<Ta: From<u8>, Tb: From<u8>, Tc: From<u8>>(
    data: &[u8],
    last_td: u8,
) -> IResult<TXn<Ta, Tb, Tc>> {
    let (data, ta) = cond(last_td & 1 << 0 > 0, be_u8)(data)?;
    let (data, tb) = cond(last_td & 1 << 1 > 0, be_u8)(data)?;
    let (data, tc) = cond(last_td & 1 << 2 > 0, be_u8)(data)?;
    let (data, td) = map(cond(last_td & 1 << 3 > 0, be_u8), |v| v.map(|v| v.into()))(data)?;
    Ok((
        data,
        TXn {
            ta: ta.map(|v| v.into()),
            tb: tb.map(|v| v.into()),
            tc: tc.map(|v| v.into()),
            td,
        },
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ATR {
    /// Electrical transmission convention (hi=1 or lo=1).
    pub ts: TS,
    /// K (number of historical bytes), whether the t(x)1 bytes are present.
    pub t0: T0,

    /// Global hardware flags. These are all handled by the reader/driver.
    /// TA1: Timing modifier. TB2: Voltage modifier. (Deprecated since 2006)
    /// TC1: Extra guard time. TD1: Presence of TX2.
    pub tx1: TXn<u8, u8, u8>,
    /// TA2: Mode negotiation. TB2: Voltage modifier. (Deprecated since 2006)
    /// TC2: Leading edge time. (T=0 only) TD2: Protocol support + TX2 presence.
    pub tx2: TXn<u8, u8, u8>,
    /// TA3: T=1 IFS. TB3: T=1 CWI. TC3: T=1 Error detection code.
    /// TD3: Protocol support. No further TXn fields should be present.
    pub tx3: TXn<u8, u8, u8>,

    /// Historical bytes. A Compact TLV value. (TODO: Parse this.)
    pub historical_bytes: Vec<u8>,

    /// Checksum byte. (We trust the reader to validate this.)
    pub tck: u8,
}

pub fn parse(data: &[u8]) -> crate::Result<ATR> {
    let (data, ts) = be_u8(data).map(|(i, v)| (i, v.into()))?;
    let (data, t0): (_, T0) = be_u8(data).map(|(i, v)| (i, v.into()))?;
    let (data, tx1) = parse_txn(data, t0.tx1)?;
    let (data, tx2) = parse_txn(data, tx1.td.map(|v| v.txn).unwrap_or_default())?;
    let (data, tx3) = parse_txn(data, tx2.td.map(|v| v.txn).unwrap_or_default())?;
    // TX4 is not a real thing as of writing and should not be here.
    assert!(tx3.td.map(|v| v.txn).unwrap_or_default() == 0x00);

    let (data, historical_bytes): (_, Vec<u8>) =
        take(t0.k)(data).map(|(i, v)| (i, v.to_owned()))?;
    let (_, tck) = be_u8(data)?;

    Ok(ATR {
        ts,
        t0,
        tx1,
        tx2,
        tx3,
        historical_bytes,
        tck,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_curve() {
        // ATR from a 2018 Curve (UK, Gemalto) card.
        let atr = parse(&[
            0x3B, 0x8E, 0x80, 0x01, 0x80, 0x31, 0x80, 0x66, 0xB1, 0x84, 0x0C, 0x01, 0x6E, 0x01,
            0x83, 0x00, 0x90, 0x00, 0x1C,
        ])
        .expect("couldn't parse ATR");
        assert_eq!(
            atr,
            ATR {
                ts: TS::Direct,
                t0: T0 {
                    tx1: 0b0000_1000,
                    k: 14,
                },
                tx1: TXn {
                    ta: None,
                    tb: None,
                    tc: None,
                    td: Some(TDn {
                        protocol: Protocol::T0,
                        txn: 0b1000,
                    }),
                },
                tx2: TXn {
                    ta: None,
                    tb: None,
                    tc: None,
                    td: Some(TDn {
                        protocol: Protocol::T1,
                        txn: 0x00,
                    }),
                },
                tx3: TXn::default(),
                historical_bytes: vec![
                    0x80, 0x31, 0x80, 0x66, 0xB1, 0x84, 0x0C, 0x01, 0x6E, 0x01, 0x83, 0x00, 0x90,
                    0x00
                ],
                tck: 0x1C,
            }
        );
    }
}
