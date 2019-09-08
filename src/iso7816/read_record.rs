use crate::errors::{Error, ErrorKind};
use crate::{Card, Command, Status, APDU, RAPDU};
use std::convert::{From, TryFrom};
use std::marker::PhantomData;

pub enum Record {
    Num(u8),
}

impl Record {
    fn p1(&self) -> u8 {
        match self {
            Self::Num(num) => *num,
        }
    }

    fn p2(&self) -> u8 {
        match self {
            Self::Num(_) => 0b0000_0100,
        }
    }
}

pub struct ReadRecord<R: TryFrom<RAPDU> = ()> {
    pub sfi: u8,
    pub record: Record,
    _response: PhantomData<R>,
}

impl<R: TryFrom<RAPDU>> ReadRecord<R> {
    pub fn new(sfi: u8, record: Record) -> Self {
        Self {
            sfi,
            record,
            _response: PhantomData::default(),
        }
    }

    pub fn num(sfi: u8, num: u8) -> Self {
        Self::new(sfi, Record::Num(num))
    }
}

impl<R: TryFrom<RAPDU>> Command for ReadRecord<R> {
    type Response = R;
}

impl<R: TryFrom<RAPDU>> Into<APDU> for ReadRecord<R> {
    fn into(self) -> APDU {
        APDU::new(
            0x00,
            0xB2,
            self.record.p1(),
            self.sfi << 3 | self.record.p2(),
            vec![],
        )
    }
}

pub struct RecordIter<'a, C: Card, R: TryFrom<RAPDU>, E = Error> {
    pub card: &'a C,
    pub sfi: u8,
    pub num: u8,
    _response: PhantomData<R>,
    _error: PhantomData<E>,
}

impl<'a, C: Card, R: TryFrom<RAPDU>, E> RecordIter<'a, C, R, E> {
    pub fn new(card: &'a C, sfi: u8) -> Self {
        Self {
            card,
            sfi,
            num: 1,
            _response: PhantomData::default(),
            _error: PhantomData::default(),
        }
    }
}
impl<'a, C: Card, R: TryFrom<RAPDU>, E> Iterator for RecordIter<'a, C, R, E>
where
    Error: From<<R as TryFrom<RAPDU>>::Error>,
    E: From<<R as TryFrom<RAPDU>>::Error>,
    E: From<Error>,
{
    type Item = Result<R, E>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.card.call(ReadRecord::num(self.sfi, self.num));
        self.num += 1;
        if let Err(Error(ErrorKind::APDU(Status::RecordNotFound), _)) = res {
            return None;
        }
        Some(res.map_err(|e| e.into()))
    }
}
