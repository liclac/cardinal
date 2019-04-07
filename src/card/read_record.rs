use crate::cmd::{Request, Response};
use crate::refs::RecordRef;
use std::marker::PhantomData;

pub struct ReadRecord<RT: Response> {
    pub rec: RecordRef,
    pub _phantom_rt: PhantomData<RT>,
}

impl<RT: Response> ReadRecord<RT> {
    pub fn new(rec: RecordRef) -> Self {
        Self {
            rec,
            _phantom_rt: PhantomData {},
        }
    }

    pub fn num(sfi: u8, num: u8) -> Self {
        Self::new(RecordRef::num(sfi, num))
    }

    // P1 is always a record number.
    pub fn p1(&self) -> u8 {
        match self.rec {
            RecordRef::Number { sfi: _, num } => num,
        }
    }

    // First 5b of P2 are the SFI of the parent file. Last 3b are flags.
    pub fn p2(&self) -> u8 {
        match self.rec {
            RecordRef::Number { sfi, num: _ } => (sfi << 3) | 0b100,
        }
    }
}

impl<RT: Response> Request for ReadRecord<RT> {
    type Returns = RT;

    fn ins(&self) -> u8 {
        0xB2
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (self.p1(), self.p2(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_number() {
        let (p1, p2, data) = ReadRecord::<()>::num(1, 1).data();
        assert_eq!(p1, 0x01);
        assert_eq!(p2, 0b00001100);
        assert_eq!(data.len(), 0);
    }
}
