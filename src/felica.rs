// Note to self:
// Okay, the ACR 1252-U docs has a pseudo-APDU for "talk to suica" (FF 00 00 00 Lc [command]).
// I don't know how portable this is, but I'm assuming it's a CCID thing. That spec is a mess.
//
// It has a single example I've managed to reproduce with opensc-tool:
// - (Identify card from ATR).
//
// - Read the IDm (same pAPDU as reading the ISO contactless CID):
//     $ opensc-tool -s 'FF CA 00 00 00'
//     Using reader with a card: ACS ACR1252 Reader [ACR1252 Reader PICC] 00 00
//     Sending: FF CA 00 00 00
//     Received (SW1=0x90, SW2=0x00):
//     01 01 0A 10 8E 1B AD 39 .......9
//
// - Use that IDm to send it a command (wtf did I just ask it for?)
//     $ opensc-tool -s 'FF 00 00 00 10 10 06 01 01 0A 10 8E 1B AD 39 01 09 01 01 80 00'
//     Using reader with a card: ACS ACR1252 Reader [ACR1252 Reader PICC] 00 00
//     Sending: FF 00 00 00 10 10 06 01 01 0A 10 8E 1B AD 39 01 09 01 01 80 00
//     Received (SW1=0x90, SW2=0x00):
//     0C 07 01 01 0A 10 8E 1B AD 39 01 A6 .........9..
//
// Okay, so that's CLS=FF, CMD=00, P1=00, P2=00 (FeliCa wrapper pAPDU).
// Lc=0x10 (16), FeliCa payload length is also 0x10 (16)? I guess it includes itself.
// Command 0x06 (Read Without Encryption), the IDm for targeting, then 01 09 01 01 80 00.
//
// 0x06 Read Without Encryption is documented in the FeliCa Users' Manual, Section 4.4.5.
// Structure:
//   Command Code [1] = 0x06
//   IDm          [8]
//   Service Num. [1] => m
//   Service List [2m] (u16, repeated service_num times)
//   Block Num.   [1] => n
//   Block List   [N] (2-3 bytes each, repeated block_num times)
//
// So I asked it to read 1 service (0x0901), 1 block (0x80, 0x00).
//
// Response: 0C 07 01 01 0A 10 8E 1B AD 39 01 A6

use crate::{util, Result};
use nom::bytes::complete::take;
use nom::number::complete::{be_u64, le_u8};
use num_enum::{FromPrimitive, IntoPrimitive};
use scroll::ctx::TryIntoCtx;
use scroll::{Pread, Pwrite, BE, LE};
use tracing::{trace, trace_span};

pub type IResult<'a, T> = nom::IResult<&'a [u8], T>;

/// Parses a CID retrieved from PCSC into an IDm.
/// In other words, casts an 8-byte &[u8] into an u64.
pub fn cid_to_idm(cid: &[u8]) -> Result<u64> {
    Ok(cid.pread_with(0, BE)?)
}

pub struct Session<'c> {
    pub card: &'c mut pcsc::Card,
}

impl<'c> Session<'c> {
    pub fn start(card: &'c mut pcsc::Card) -> Result<Self> {
        let span = trace_span!("Session");
        let _enter = span.enter();

        let mut slf = Self { card };
        trace!("Starting Transparent Session, enabling RF Field...");
        slf.tcall(0x00, &[0x81, 0x00, 0x84, 0x00])?; // Start Transparent Session, Turn On RF Field
        trace!("Switching Protocol to FeliCa...");
        slf.tcall(0x02, &[0x8F, 0x02, 0x03, 0x00])?; // Switch Protocol: FeliCa, No Layering.
        trace!("Setting TX/RX flags...");
        slf.tcall(0x01, &[0x90, 0x02, 0x00, 0x00])?; // Set TX/RX flags.

        Ok(slf)
    }

    pub fn end(&mut self) -> Result<()> {
        let span = trace_span!("Session");
        let _enter = span.enter();

        trace!("Ending FeliCa Session...");
        self.tcall(0x00, &[0x82, 0x00])?; // End Transparent Session
        Ok(())
    }

    fn tcall(&mut self, p2: u8, payload: &[u8]) -> Result<()> {
        let mut rbuf = [0u8; 16];
        util::expect_pcsc_transparent_succ(util::call_apdu(
            self.card,
            &mut [0u8; 16],
            &mut rbuf,
            apdu::Command::new_with_payload(0xFF, 0xC2, 0x00, p2, payload),
        )?)?;
        Ok(())
    }
}

impl<'c> Drop for Session<'c> {
    fn drop(&mut self) {
        self.end().expect("couldn't end FeliCa session on drop");
    }
}

pub trait Command<'a>: Sized + TryIntoCtx
where
    <Self as TryIntoCtx>::Error: From<scroll::Error>,
    crate::Error: From<<Self as TryIntoCtx>::Error>,
{
    /// Associated command code.
    const CODE: CommandCode;

    /// Associated response code.
    type Response: Response<'a>;

    /// Return an APDU wrapper.
    fn apdu<'w>(self, wbuf: &'w mut [u8]) -> Result<apdu::Command<'w>> {
        // We're in a PCSC Transparent Session, so we wrap commands in a BER-TLV blob.
        wbuf.pwrite::<u8>(0x95, 0)?; // BER: Tag=0x95 Transceive Data Object.
        wbuf.pwrite::<u8>(0x00, 1)?; // BER: Length placeholder.

        // Write the command itself, then backfill length bytes.
        let cmd_len = wbuf.pwrite(self, 3)?; // FeliCa: The command itself.
        assert!(cmd_len <= 0b0111_1111); // Sanity check the length.
        wbuf.pwrite::<u8>((cmd_len + 1) as u8, 1)?; // BER: Re-write length byte.
        wbuf.pwrite::<u8>((cmd_len + 1) as u8, 2)?; // FeliCa: Command length, including self.

        // PCSC Transparent Exchange pseudo-APDU, supported in semi-modern readers.
        let payload = &wbuf[..cmd_len + 3];
        {
            let (rest, (tag, value)) = crate::ber::parse_next(payload).unwrap();
            assert_eq!(rest, &[]);
            println!("{:02X?} => {:02X?}", tag, value);
        }
        Ok(apdu::Command::new_with_payload(
            0xFF, 0xC2, 0x00, 0x01, payload,
        ))
    }

    /// Executes the command against the given card and returns the response.
    fn call(
        self,
        sess: &mut Session,
        wbuf: &mut [u8],
        rbuf: &'a mut [u8],
    ) -> Result<Self::Response> {
        // TODO: This is a bit of a pointless extra step.
        let mut apdu_buf = [0u8; 256];
        let apdu = self.apdu(&mut apdu_buf[..])?;

        Self::Response::parse(util::expect_pcsc_transparent_succ(util::call_apdu(
            sess.card, wbuf, rbuf, apdu,
        )?)?)
    }
}

pub trait Response<'a>: Sized {
    const CODE: CommandCode;

    fn iparse(data: &'a [u8]) -> IResult<Self>;

    fn parse(data: &'a [u8]) -> Result<Self> {
        Ok(Self::iparse(data).map(|(_, v)| v)?)
    }
}

#[derive(Debug, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum CommandCode {
    ReadWithoutEncryption = 0x06,
    ReadWithoutEncryptionResponse = 0x07,
    RequestSystemCode = 0x0C,
    RequestSystemCodeResponse = 0x0D,
    #[num_enum(catch_all)]
    Unknown(u8),
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReadWithoutEncryption {
    pub idm: u64,
    pub services: Vec<u16>,
    pub blocks: Vec<BlockListElement>,
}

impl<'a> Command<'a> for &ReadWithoutEncryption {
    const CODE: CommandCode = CommandCode::ReadWithoutEncryption;
    type Response = ReadWithoutEncryptionResponse<'a>;
}

impl TryIntoCtx for &ReadWithoutEncryption {
    type Error = scroll::Error;

    fn try_into_ctx(self, wbuf: &mut [u8], _: ()) -> Result<usize, Self::Error> {
        let mut offset = 0;
        wbuf.gwrite::<u8>(Self::CODE.into(), &mut offset)?;
        wbuf.gwrite_with(self.idm, &mut offset, BE)?;
        wbuf.gwrite::<u8>(self.services.len() as u8, &mut offset)?;
        for sid in self.services.iter() {
            wbuf.gwrite_with(sid, &mut offset, LE)?;
        }
        wbuf.gwrite::<u8>(self.blocks.len() as u8, &mut offset)?;
        for bid in self.blocks.iter() {
            wbuf.gwrite(bid, &mut offset)?;
        }
        Ok(offset)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ReadWithoutEncryptionResponse<'a> {
    pub idm: u64,
    pub status: (u8, u8),
    pub blocks: Vec<&'a [u8]>,
}

impl<'a> Response<'a> for ReadWithoutEncryptionResponse<'a> {
    const CODE: CommandCode = CommandCode::ReadWithoutEncryptionResponse;

    fn iparse(rbuf: &'a [u8]) -> IResult<Self> {
        Ok((
            rbuf,
            Self {
                idm: 0,
                status: (0, 0),
                blocks: vec![],
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u16)]
pub enum SystemCode {
    /// Sytem that uses regular NFC NDEF.
    NDEF = 0x12FC,
    /// Host-based Emulation for NFC-F (HCE-F).
    HostEmulation = 0x4000,
    /// FeliCa Lite-S
    FeliCaLiteS = 0x88B4,
    /// FeliCa Secure ID
    FeliCaSecureID = 0x957A,
    /// FeliCa Networks "Common Area".
    FeliCaCommon = 0xFE00,
    /// FeliCa Plug
    FeliCaPlug = 0xFEE1,
    #[num_enum(catch_all)]
    Unknown(u16),
}

impl std::fmt::Display for SystemCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NDEF => write!(f, "NFC NDEF"),
            Self::HostEmulation => write!(f, "Host-based Emulation"),
            Self::FeliCaLiteS => write!(f, "FeliCa Lite-S"),
            Self::FeliCaSecureID => write!(f, "FeliCa Secure ID"),
            Self::FeliCaCommon => write!(f, "FeliCa Common"),
            Self::FeliCaPlug => write!(f, "FeliCa Plug"),
            Self::Unknown(v) => write!(f, "Unknown({:04X})", v),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestSystemCode {
    pub idm: u64,
}

impl<'a> Command<'a> for &RequestSystemCode {
    const CODE: CommandCode = CommandCode::RequestSystemCode;
    type Response = RequestSystemCodeResponse;
}

impl TryIntoCtx for &RequestSystemCode {
    type Error = scroll::Error;

    fn try_into_ctx(self, wbuf: &mut [u8], _: ()) -> Result<usize, Self::Error> {
        let mut offset = 0;
        wbuf.gwrite::<u8>(Self::CODE.into(), &mut offset)?;
        wbuf.gwrite_with(self.idm, &mut offset, BE)?;
        Ok(offset)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestSystemCodeResponse {
    pub idm: u64,
    pub systems: Vec<SystemCode>,
}

impl<'a> Response<'a> for RequestSystemCodeResponse {
    const CODE: CommandCode = CommandCode::RequestSystemCodeResponse;

    fn iparse(data: &'a [u8]) -> IResult<Self> {
        let (data, _) = le_u8(data)?; // Ignore length prefix.
        let (data, code) = le_u8(data)?;
        assert_eq!(Self::CODE, code.into());
        let (data, idm) = be_u64(data)?;

        let (data, num_systems) = le_u8(data)?;
        let (data, systems_data) = take(num_systems * 2)(data)?;
        let systems = systems_data
            .chunks(2)
            .map(|data| u16::from_be_bytes([data[0], data[1]]).into())
            .collect();
        Ok((data, Self { idm, systems }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum AccessMode {
    Normal = 0,
    Cashback = 1,
    #[num_enum(catch_all)]
    Unknown(u8),
}

/// A list of Block List Elements makes up a Block List. Block List Elements can have
/// 2 or 3 byte lengths (indicated by their first byte), but this type smooths this over.
///
/// To save bytes, the Service Code is transmitted separately alongside the blocklist,
/// and we reference them by indexes into that list. For example, a ReadWithoutEncryption
/// command for service 8 -> blocks 10 and 12, and service 20 -> block 22 would say:
///   services: [8, 20], blocks: [(0, 10), (0, 12), (1, 22)].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockListElement {
    /// Access Mode. Always Normal unless you're doing Cashback to a Purse service.
    pub mode: AccessMode,
    /// Index in the service list sent alongside this blocklist, max 15 (0x0F).
    pub service_idx: u8,
    /// Block number (max 3 bytes).
    pub block_num: u16,
}

impl scroll::ctx::TryIntoCtx<()> for &BlockListElement {
    type Error = scroll::Error;

    fn try_into_ctx(self, wbuf: &mut [u8], _: ()) -> Result<usize, Self::Error> {
        let mut offset = 0;
        wbuf.gwrite::<u8>(
            // 0bX---_--- is 1 if self is 2 bytes (num fits in u8), else 0 for 3 (u16).
            if self.block_num <= u8::MAX as u16 { 0b1000_0000 } else { 0b0000_0000}
            // 0b-XXX_--- is the mode. Why this needs 3 bits is anyone's guess.
            | (u8::from(self.mode) << 4)
            // 0b----_XXXX is the service code index.
            | (self.service_idx & 0b0000_1111),
            &mut offset,
        )?;
        if self.block_num <= u8::MAX as u16 {
            wbuf.gwrite::<u8>(self.block_num as u8, &mut offset)?;
        } else {
            wbuf.gwrite_with::<u16>(self.block_num, &mut offset, LE)?;
        }
        Ok(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cid_to_idm() {
        // IDm from the example in the ACR-1252U manual.
        assert_eq!(
            cid_to_idm(&[0x01, 0x01, 0x06, 0x01, 0xCB, 0x09, 0x57, 0x03]).unwrap(),
            0x01010601CB095703
        );
    }

    #[test]
    fn test_read_without_encryption() {
        // Example command from the ACR-1252U manual.
        let mut wbuf = [0u8; 256];
        let apdu = ReadWithoutEncryption {
            idm: 0x01010601CB095703,
            services: vec![0x0109],
            blocks: vec![BlockListElement {
                mode: AccessMode::Normal,
                service_idx: 0,
                block_num: 0,
            }],
        }
        .apdu(&mut wbuf)
        .unwrap();
        assert_eq!(
            (apdu.cla, apdu.ins, apdu.p1, apdu.p2, apdu.le),
            (0xFF, 0xC2, 0x00, 0x01, None)
        );
        println!("{:02X?}", apdu.payload);
        assert_eq!(
            apdu.payload.expect("no payload"),
            &[
                0x95, 0x81, 16, 16, 0x06, 0x01, 0x01, 0x06, 0x01, 0xCB, 0x09, 0x57, 0x03, 0x01,
                0x09, 0x01, 0x01, 0x80, 0x00
            ],
        );

        let mut apdu_buf = [0u8; 256];
        apdu.write(&mut apdu_buf);
        assert_eq!(
            &apdu_buf[..apdu.len()],
            &[
                0xFF, 0xC2, 0x00, 0x01, 0x13, 0x95, 0x81, 0x10, 0x10, 0x06, 0x01, 0x01, 0x06, 0x01,
                0xCB, 0x09, 0x57, 0x03, 0x01, 0x09, 0x01, 0x01, 0x80, 0x00
            ],
        );
    }

    #[test]
    fn test_request_system_code() {
        let mut wbuf = [0u8; 256];
        let apdu = RequestSystemCode {
            idm: 0x1122334455667788,
        }
        .apdu(&mut wbuf)
        .unwrap();
        assert_eq!(
            (apdu.cla, apdu.ins, apdu.p1, apdu.p2, apdu.le),
            (0xFF, 0xC2, 0x00, 0x01, None)
        );
        println!("{:02X?}", apdu.payload);
        assert_eq!(
            apdu.payload.expect("no payload"),
            &[0x95, 0x81, 10, 10, 0x0C, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
        );
    }

    #[test]
    fn test_request_system_code_response() {
        assert_eq!(
            RequestSystemCodeResponse::parse(&[
                0x0F, 0x0D, 0x01, 0x01, 0x0A, 0x10, 0x8E, 0x1B, 0xAD, 0x39, 0x02, 0x00, 0x03, 0xFE,
                0x00,
            ])
            .unwrap(),
            RequestSystemCodeResponse {
                idm: 0x01010A108E1BAD39,
                systems: vec![SystemCode::Unknown(0x0003), SystemCode::FeliCaCommon],
            },
        )
    }
}
