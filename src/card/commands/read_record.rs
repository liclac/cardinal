use crate::core::{Request, Response};
use std::marker::PhantomData;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Record {
    Number { sfi: u8, num: u8 },
}

impl Record {
    pub fn num(sfi: u8, num: u8) -> Record {
        Record::Number { sfi, num }
    }

    // P1 is always a record number.
    pub fn p1(&self) -> u8 {
        match self {
            &Record::Number { sfi: _, num } => num,
        }
    }

    // First 5b of P2 are the SFI of the parent file. Last 3b are flags.
    pub fn p2(&self) -> u8 {
        match self {
            &Record::Number { sfi, num: _ } => (sfi << 3) | 0b100,
        }
    }
}

pub struct ReadRecord<RT: Response> {
    pub rec: Record,
    pub _phantom_rt: PhantomData<RT>,
}

impl<RT: Response> ReadRecord<RT> {
    pub fn new(rec: Record) -> Self {
        Self {
            rec,
            _phantom_rt: PhantomData {},
        }
    }
}

impl<RT: Response> Request for ReadRecord<RT> {
    type Returns = RT;

    fn ins(&self) -> u8 {
        0xB2
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (self.rec.p1(), self.rec.p2(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_num() {
        let rec = Record::num(1, 1);
        assert_eq!(rec.p1(), 0x01);
        assert_eq!(rec.p2(), 0b00001100);
    }
}
