use crate::core::apdu;
use crate::errors::Result;
use crate::transport::protocol::Protocol;

pub struct APDU();

impl Protocol for APDU {
    fn serialize_req(req: &apdu::Request) -> Result<Vec<u8>> {
        // The header is always fixed...
        let mut bin = vec![req.cla, req.ins, req.p1, req.p2];

        // command length (Lc) + data: [Lc: u8][data: Lc], [Lc=0] -> following bytes [Lc2: u16],
        // [Lc2=0] -> [Lc2=65536], data longer than that will be rejected.
        match req.data.len() {
            0 => (),
            x @ 1...255 => bin.push(x as u8),
            256 => bin.push(0x00),
            x => bail!("apdu command data is too long: {}", x),
        };
        // bin.append(&mut req.data); // This empties req.data.
        for b in req.data.iter() {
            bin.push(*b);
        }

        // TODO: Fix extended Les.
        bin.push(req.le.unwrap_or(255) as u8); // Le = expected (maximum) length of response.
        Ok(bin)
    }

    fn deserialize_res(data: &[u8]) -> Result<apdu::Response> {
        if data.len() < 2 {
            bail!("response data is too short")
        }
        Ok(apdu::Response::new(
            apdu::Status(data[data.len() - 2], data[data.len() - 1]),
            &data[0..data.len() - 2],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::apdu::{Request, Response, Status};

    #[test]
    fn test_serialize_req() {
        assert_eq!(
            APDU::serialize_req(&Request::new(
                0x12,
                0x34,
                0x56,
                0x78,
                vec![0x9A, 0xBC, 0xDE, 0xEF]
            ),)
            .unwrap(),
            vec![0x12, 0x34, 0x56, 0x78, 0x04, 0x9A, 0xBC, 0xDE, 0xEF, 0xFF],
        );
    }

    #[test]
    fn test_serialize_req_expect() {
        assert_eq!(
            APDU::serialize_req(
                &Request::new(0x12, 0x34, 0x56, 0x78, vec![0x9A, 0xBC, 0xDE, 0xEF]).expect(0x69)
            )
            .unwrap(),
            vec![0x12, 0x34, 0x56, 0x78, 0x04, 0x9A, 0xBC, 0xDE, 0xEF, 0x69],
        );
    }

    #[test]
    fn test_deserialise_res_empty() {
        assert_eq!(
            APDU::deserialize_res(&[]).unwrap_err().description(),
            "response data is too short"
        )
    }

    #[test]
    fn test_deserialise_res_status_only() {
        assert_eq!(
            APDU::deserialize_res(&[0x90, 0x00]).unwrap(),
            Response::new(Status(0x90, 0x00), vec![])
        );
    }

    #[test]
    fn test_deserialise_res_data() {
        assert_eq!(
            APDU::deserialize_res(&[0x69, 0x42, 0x00, 0x90, 0x00]).unwrap(),
            Response::new(Status(0x90, 0x00), vec![0x69, 0x42, 0x00])
        );
    }
}
