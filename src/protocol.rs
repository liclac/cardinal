use crate::errors::{ErrorKind, Result};
use crate::util;
use crate::{Status, APDU, RAPDU};
use std::convert::TryInto;
use std::io::prelude::*;

/// Abstraction around smartcard wire protocols.
/// TODO: Implement T=CL, I need to find a viable transport layer for NFC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    T0,
    T1,
}

impl Protocol {
    pub fn write_req<W: Write>(&self, w: &mut W, req: &APDU) -> Result<usize> {
        let mut num = util::write_all(w, &[req.cla, req.ins, req.p1, req.p2])?;
        if req.data.len() > 0 {
            num += util::write_all(
                w,
                &[req.data.len().try_into().map_err(|_| {
                    ErrorKind::APDUBodyTooLong(req.data.len(), u8::max_value() as usize)
                })?],
            )?;
            num += util::write_all(w, &req.data)?;
        }
        if self == &Self::T1 || req.data.len() == 0 {
            num += util::write_all(w, &[req.le])?;
        }
        Ok(num)
    }

    pub fn decode_res<'a>(&self, data: &'a [u8]) -> Result<RAPDU> {
        let (sw2, data) = data.split_last().ok_or("data truncated: no SW2")?;
        let (sw1, data) = data.split_last().ok_or("data truncated: no SW1")?;
        Ok(RAPDU {
            sw: Status::from(*sw1, *sw2),
            data: data.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::Error;

    #[test]
    fn t0_write_req() -> Result<()> {
        let mut buf = Vec::new();
        Protocol::T0.write_req(&mut buf, &APDU::new(0x00, 0xA4, 0x12, 0x34, vec![]))?;
        assert_eq!(&buf, &[0x00, 0xA4, 0x12, 0x34, 0x00],);
        Ok(())
    }

    #[test]
    fn t0_write_req_body() -> Result<()> {
        let mut buf = Vec::new();
        Protocol::T0.write_req(
            &mut buf,
            &APDU::new(0x00, 0xA4, 0x12, 0x34, vec![0x56, 0x78]),
        )?;
        assert_eq!(&buf, &[0x00, 0xA4, 0x12, 0x34, 0x02, 0x56, 0x78],);
        Ok(())
    }

    #[test]
    fn t1_write_req() -> Result<()> {
        let mut buf = Vec::new();
        Protocol::T1.write_req(&mut buf, &APDU::new(0x00, 0xA4, 0x12, 0x34, vec![]))?;
        assert_eq!(&buf, &[0x00, 0xA4, 0x12, 0x34, 0x00],);
        Ok(())
    }

    #[test]
    fn t1_write_req_body() -> Result<()> {
        let mut buf = Vec::new();
        Protocol::T1.write_req(
            &mut buf,
            &APDU::new(0x00, 0xA4, 0x12, 0x34, vec![0x56, 0x78]),
        )?;
        assert_eq!(&buf, &[0x00, 0xA4, 0x12, 0x34, 0x02, 0x56, 0x78, 0x00],);
        Ok(())
    }

    #[test]
    fn t1_write_req_body_too_long() -> Result<()> {
        let body: Vec<u8> = std::iter::repeat(0x69).take(512).collect();
        let mut buf = Vec::new();
        match Protocol::T1
            .write_req(&mut buf, &APDU::new(0x00, 0xA4, 0x12, 0x34, body))
            .unwrap_err()
        {
            Error(ErrorKind::APDUBodyTooLong(512, 255), _) => assert!(true),
            v => assert!(false, "wrong error: {}", v),
        };
        Ok(())
    }

    #[test]
    fn t0_decode_res() -> Result<()> {
        let res = Protocol::T0.decode_res(&[0x90, 0x00])?;
        assert_eq!(
            &res,
            &RAPDU {
                data: vec![],
                sw: Status::OK,
            }
        );
        Ok(())
    }

    #[test]
    fn t0_decode_res_body() -> Result<()> {
        let res = Protocol::T0.decode_res(&[0x12, 0x34, 0x56, 0x78, 0x90, 0x00])?;
        assert_eq!(
            &res,
            &RAPDU {
                data: vec![0x12, 0x34, 0x56, 0x78],
                sw: Status::OK,
            }
        );
        Ok(())
    }

    #[test]
    fn t1_decode_res() -> Result<()> {
        let res = Protocol::T1.decode_res(&[0x90, 0x00])?;
        assert_eq!(
            &res,
            &RAPDU {
                data: vec![],
                sw: Status::OK,
            }
        );
        Ok(())
    }

    #[test]
    fn t1_decode_res_body() -> Result<()> {
        let res = Protocol::T1.decode_res(&[0x12, 0x34, 0x56, 0x78, 0x90, 0x00])?;
        assert_eq!(
            &res,
            &RAPDU {
                data: vec![0x12, 0x34, 0x56, 0x78],
                sw: Status::OK,
            }
        );
        Ok(())
    }
}
