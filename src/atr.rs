//! ATR (Answer-to-Reset) parser.
//!
//! Aside from ISO 7816, this is covered by the EMV L1 Contact Interface Specification,
//! Section 8: "Answer to Reset", which is freely available from EMVCo's website.
//! For ease of access, this is written using the EMV specs, as well as the surprisingly
//! complete Wikipedia page: https://en.wikipedia.org/wiki/Answer_to_reset
//!
//! Useful online ATR parser: https://smartcard-atr.apdu.fr/

use std::fmt::Display;

use nom::bytes::complete::take;
use nom::combinator::{cond, map};
use nom::number::complete::{be_u16, be_u32, be_u8};
use num_enum::{FromPrimitive, IntoPrimitive};
use tracing::{trace_span, warn};

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

impl From<T0> for u8 {
    fn from(t0: T0) -> Self {
        t0.k | (t0.tx1 << 4)
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

impl From<TDn> for u8 {
    fn from(tdn: TDn) -> Self {
        u8::from(tdn.protocol) | (tdn.txn << 4)
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
pub enum HistoricalBytes {
    Status(HistoricalBytesStatus),
    TLV(HistoricalBytesTLV),
    Unknown(u8, Vec<u8>),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HistoricalBytesTLV {
    pub raw: Vec<u8>,
    pub service_data: Option<u8>,
    pub initial_access: Option<InitialAccess>,
    pub pre_issuing_data: Option<Vec<u8>>,
    pub status: Option<HistoricalBytesStatus>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct HistoricalBytesStatus {
    pub status: Option<u8>,
    pub sw1sw2: Option<u16>,
}

fn parse_historical_bytes_status(data: &[u8]) -> Option<HistoricalBytesStatus> {
    match data.len() {
        1 => Some(HistoricalBytesStatus {
            status: Some(data[0]),
            sw1sw2: None,
        }),
        2 => Some(HistoricalBytesStatus {
            status: None,
            sw1sw2: Some(u16::from_be_bytes([data[0], data[1]])),
        }),
        3 => Some(HistoricalBytesStatus {
            status: Some(data[0]),
            sw1sw2: Some(u16::from_be_bytes([data[1], data[2]])),
        }),
        _ => {
            warn!("invalid status: {:02X?}", data);
            None
        }
    }
}

/// 0x4Y "Initial access bytes" inside the Historical Bytes.
///
/// I'm genuinely unsure about the proper spec for this - I think it's in PC/SC, but the
/// PC/SC specifications are incomprehensible cryptids and I can never even tell if I'm
/// reading the right document. This is just based on the docs for my ACR 1252-U reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitialAccess {
    /// Registered Application Provider Identifier (RID), eg. A0 00 00 03 06.
    pub rid: Provider,
    /// Standard in use.
    pub standard: Standard,
    /// Card name.
    pub card_name: CardName,
    /// Unused, reserved for future use.
    pub rfu: u32,
}

fn parse_initial_access(data: &[u8]) -> IResult<InitialAccess> {
    let (data, rid) = map(take(5usize), |v: &[u8]| match v {
        PROVIDER_ID_PCSC_WORKGROUP => Provider::PCSCWorkgroup,
        _ => Provider::Unknown(v.to_owned()),
    })(data)?;
    let (data, standard) = map(be_u8, |v| v.into())(data)?;
    let (data, card_name) = map(be_u16, |v| v.into())(data)?;
    let (data, rfu) = be_u32(data)?;
    Ok((
        data,
        InitialAccess {
            rid,
            standard,
            card_name,
            rfu,
        },
    ))
}

const PROVIDER_ID_PCSC_WORKGROUP: &[u8] = &[0xA0, 0x00, 0x00, 0x03, 0x06];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    PCSCWorkgroup,
    Unknown(Vec<u8>),
}

impl Provider {
    pub fn id(&self) -> &[u8] {
        match self {
            Self::PCSCWorkgroup => PROVIDER_ID_PCSC_WORKGROUP,
            Self::Unknown(v) => &v,
        }
    }
}

impl Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PCSCWorkgroup => write!(f, "PC/SC Workgroup"),
            Self::Unknown(v) => write!(f, "Unknown({})", hex::encode_upper(v)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum Standard {
    Iso14443a3 = 0x03,
    FeliCa = 0x11,
    #[num_enum(catch_all)]
    Unknown(u8),
}

impl Display for Standard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Iso14443a3 => write!(f, "ISO 14443"),
            Self::FeliCa => write!(f, "FeliCa"),
            Self::Unknown(v) => write!(f, "Unknown({:02X})", v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum CardName {
    MifareClassic1K = 0x0001,
    MifareClassic4K = 0x0002,
    MifareUltralight = 0x0003,
    MifareMini = 0x0026,
    MifareUltralightC = 0x003A,
    MifarePlusSL12K = 0x0036,
    MifarePlusSL14K = 0x0037,
    MifarePlusSL22K = 0x0038,
    MifarePlusSL24K = 0x0039,
    TopazJewel = 0x0030,
    FeliCa = 0x003B,
    JCOP30 = 0xFF28,
    SRIX = 0x0007,
    #[num_enum(catch_all)]
    Unknown(u16),
}

impl Display for CardName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MifareClassic1K => write!(f, "MIFARE Classic 1K"),
            Self::MifareClassic4K => write!(f, "MIFARE Classic 4K"),
            Self::MifareUltralight => write!(f, "MIFARE Ultralight"),
            Self::MifareMini => write!(f, "MIFARE Mini"),
            Self::MifareUltralightC => write!(f, "MIFARE Ultralight C"),
            Self::MifarePlusSL12K => write!(f, "MIFARE Plus SL1 2K"),
            Self::MifarePlusSL14K => write!(f, "MIFARE Plus SL1 4K"),
            Self::MifarePlusSL22K => write!(f, "MIFARE Plus SL2 2K"),
            Self::MifarePlusSL24K => write!(f, "MIFARE Plus SL2 4K"),
            Self::TopazJewel => write!(f, "Topaz/Jewel"),
            Self::FeliCa => write!(f, "FeliCa"),
            Self::JCOP30 => write!(f, "JCOP 30"),
            Self::SRIX => write!(f, "SRIX"),
            Self::Unknown(v) => write!(f, "Unknown({:04X})", v),
        }
    }
}

fn parse_historical_bytes<'a>(data: &'a [u8]) -> IResult<HistoricalBytes> {
    let span = trace_span!("HistoricalBytes");
    let _enter = span.enter();

    match be_u8(data)? {
        (data, tag @ 0x10) => Ok((
            &data[data.len()..],
            if let Some(status) = parse_historical_bytes_status(data) {
                HistoricalBytes::Status(status)
            } else {
                HistoricalBytes::Unknown(tag, data.to_owned())
            },
        )),
        (data, 0x80) => Ok({
            let mut tlv = HistoricalBytesTLV::default();
            tlv.raw = data.to_owned();

            let mut rest = data;
            while rest.len() > 0 {
                // This isn't BER, this is COMPACT-TLV. High nibble is a tag, low is a length.
                // Thankfully, this makes the parser nice and compact, too.
                let (data, (tag, length)) =
                    map(be_u8, |tl| (tl & 0b1111_0000, tl & 0b0000_1111))(rest)?;

                // ...except when the length is F and the next byte is the real length?
                // (I can only find this mentioned in the docs for my ACR 1252-U reader.)
                let (data, length) = if length != 0xF {
                    (data, length)
                } else {
                    be_u8(data)?
                };

                let (data, value) = take(length)(data)?;
                match tag {
                    0x30 => tlv.service_data = value.first().copied(),
                    0x40 => {
                        tlv.initial_access = parse_initial_access(value)
                            .map_err(|err| {
                                warn!("couldn't parse initial access bytes");
                                err
                            })
                            .map(|(_, v)| v)
                            .ok()
                    }
                    0x60 => tlv.pre_issuing_data = Some(value.to_owned()),
                    0x80 => tlv.status = parse_historical_bytes_status(value),
                    _ => warn!("unknown tag: {:02X} => {:02X?}", tag, value),
                }
                rest = data;
            }
            (data, HistoricalBytes::TLV(tlv))
        }),
        (data, cat) => Ok((
            &data[data.len()..],
            HistoricalBytes::Unknown(cat, data.to_owned()),
        )),
    }
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

    /// Historical bytes.
    pub historical_bytes: Option<HistoricalBytes>,

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

    let (data, historical_bytes) = if t0.k > 0 {
        let (data, rawhb) = take(t0.k)(data)?;
        let (_, hb) = parse_historical_bytes(rawhb)?;
        (data, Some(hb))
    } else {
        (data, None)
    };
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
    fn test_t0_u8() {
        assert_eq!(u8::from(T0::from(0x8E)), 0x8E);
    }

    #[test]
    fn test_tdn_u8() {
        assert_eq!(u8::from(TDn::from(0x81)), 0x81);
    }

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
                historical_bytes: Some(HistoricalBytes::TLV(HistoricalBytesTLV {
                    raw: vec![
                        0x31, 0x80, 0x66, 0xB1, 0x84, 0x0C, 0x01, 0x6E, 0x01, 0x83, 0x00, 0x90,
                        0x00
                    ],
                    service_data: Some(0x80),
                    initial_access: None,
                    pre_issuing_data: Some(vec![0xB1, 0x84, 0x0C, 0x01, 0x6E, 0x01]),
                    status: Some(HistoricalBytesStatus {
                        status: Some(0x00),
                        sw1sw2: Some(0x9000)
                    }),
                })),
                tck: 0x1C,
            }
        );
    }

    #[test]
    fn test_parse_pasmo() {
        // ATR from a 2019 PASMO (FeliCa) card.
        let atr = parse(&[
            0x3B, 0x8F, 0x80, 0x01, 0x80, 0x4F, 0x0C, 0xA0, 0x00, 0x00, 0x03, 0x06, 0x11, 0x00,
            0x3B, 0x00, 0x00, 0x00, 0x00, 0x42,
        ])
        .expect("couldn't parse ATR");
        assert_eq!(
            atr,
            ATR {
                ts: TS::Direct,
                t0: T0 { tx1: 0b1000, k: 15 },
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
                        txn: 0b0000,
                    }),
                },
                tx3: TXn::default(),
                historical_bytes: Some(HistoricalBytes::TLV(HistoricalBytesTLV {
                    raw: vec![
                        0x4F, 0x0C, 0xA0, 0x00, 0x00, 0x03, 0x06, 0x11, 0x00, 0x3B, 0x00, 0x00,
                        0x00, 0x00
                    ],
                    initial_access: Some(InitialAccess {
                        rid: Provider::PCSCWorkgroup,
                        standard: Standard::FeliCa,
                        card_name: CardName::FeliCa,
                        rfu: 0x00000000,
                    }),
                    service_data: None,
                    pre_issuing_data: None,
                    status: None,
                })),
                tck: 0x42,
            }
        );
    }
}
