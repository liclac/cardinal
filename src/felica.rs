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
use pcsc::Card;
use scroll::{Pread, Pwrite, BE, LE};

pub type IResult<'a, T> = nom::IResult<&'a [u8], T>;

/// Parses a CID retrieved from PCSC into an IDm.
/// In other words, casts an 8-byte &[u8] into an u64.
pub fn cid_to_idm(cid: &[u8]) -> Result<u64> {
    Ok(cid.pread_with(0, BE)?)
}

pub trait Command<'a> {
    /// Associated command code.
    const CODE: CommandCode;

    /// Associated response code.
    type Response: Response<'a>;

    /// Write the command (including length prefix!) to the buffer and return a slice of it.
    fn write<'w>(&self, wbuf: &'w mut [u8]) -> Result<&'w [u8]>;

    /// Return an APDU wrapper.
    fn apdu<'w>(&self, wbuf: &'w mut [u8]) -> Result<apdu::Command<'w>> {
        let payload = self.write(wbuf)?;
        debug_assert_eq!(payload[0] as usize, payload.len());
        // Note: Although technically correct, setting an Le here will break the command.
        Ok(apdu::Command::new_with_payload(
            0xFF, 0x00, 0x00, 0x00, payload,
        ))
    }

    /// Executes the command against the given card and returns the response.
    fn call(&self, card: &mut Card, wbuf: &mut [u8], rbuf: &'a mut [u8]) -> Result<Self::Response> {
        // TODO: This is a bit of a pointless extra step.
        let mut apdu_buf = [0u8; 256];
        let apdu = self.apdu(&mut apdu_buf[..])?;

        Self::Response::parse(util::call_apdu(card, wbuf, rbuf, apdu)?)
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
    // Blocklist encoding is an entire adventure I'm *not* getting into here.
    pub blocks: Vec<u16>,
}

impl<'a> Command<'a> for ReadWithoutEncryption {
    const CODE: CommandCode = CommandCode::ReadWithoutEncryption;
    type Response = ReadWithoutEncryptionResponse<'a>;

    fn write<'w>(&self, wbuf: &'w mut [u8]) -> Result<&'w [u8]> {
        let mut offset = 1;
        wbuf.gwrite::<u8>(Self::CODE.into(), &mut offset)?;
        wbuf.gwrite_with(self.idm, &mut offset, BE)?;
        wbuf.gwrite::<u8>(self.services.len() as u8, &mut offset)?;
        for sid in self.services.iter() {
            wbuf.gwrite_with(sid, &mut offset, LE)?;
        }
        wbuf.gwrite::<u8>(self.blocks.len() as u8, &mut offset)?;
        for bid in self.blocks.iter() {
            wbuf.gwrite_with(bid, &mut offset, LE)?;
        }
        wbuf[0] = offset as u8;
        Ok(&wbuf[..offset])
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

#[derive(Debug, PartialEq, Eq)]
pub struct RequestSystemCode {
    pub idm: u64,
}

impl<'a> Command<'a> for RequestSystemCode {
    const CODE: CommandCode = CommandCode::RequestSystemCode;
    type Response = RequestSystemCodeResponse;

    fn write<'w>(&self, wbuf: &'w mut [u8]) -> Result<&'w [u8]> {
        let mut offset = 1;
        wbuf.gwrite::<u8>(Self::CODE.into(), &mut offset)?;
        wbuf.gwrite_with(self.idm, &mut offset, BE)?;
        wbuf[0] = offset as u8;
        Ok(&wbuf[..offset])
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestSystemCodeResponse {
    pub idm: u64,
    pub systems: Vec<u16>,
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
            .map(|data| u16::from_le_bytes([data[0], data[1]]))
            .collect();
        Ok((data, Self { idm, systems }))
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
            blocks: vec![0x80],
        }
        .apdu(&mut wbuf)
        .unwrap();
        assert_eq!(
            (apdu.cla, apdu.ins, apdu.p1, apdu.p2, apdu.le),
            (0xFF, 0x00, 0x00, 0x00, None)
        );
        assert_eq!(
            apdu.payload.expect("no payload"),
            &[
                0x10, 0x06, 0x01, 0x01, 0x06, 0x01, 0xCB, 0x09, 0x57, 0x03, 0x01, 0x09, 0x01, 0x01,
                0x80, 0x00
            ],
        );

        let mut apdu_buf = [0u8; 256];
        apdu.write(&mut apdu_buf);
        assert_eq!(
            &apdu_buf[..apdu.len()],
            &[
                0xFF, 0x00, 0x00, 0x00, 0x10, 0x10, 0x06, 0x01, 0x01, 0x06, 0x01, 0xCB, 0x09, 0x57,
                0x03, 0x01, 0x09, 0x01, 0x01, 0x80, 0x00
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
            (0xFF, 0x00, 0x00, 0x00, None)
        );
        assert_eq!(
            apdu.payload.expect("no payload"),
            &[10, 0x0C, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]
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
                systems: vec![0x0300, 0x00FE],
            },
        )
    }
}
