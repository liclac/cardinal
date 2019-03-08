use crate::cmd::{Request, Response};
use crate::refs::FileRef;

// A SELECT command can select the first, last, next or previous occurrence of an ID.
// Normally, what you want is the first; we should build an iterator API around the rest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectOccurrence {
    First,
    Last,
    Next,
    Previous,
}

impl SelectOccurrence {
    pub fn apdu_p2(&self) -> u8 {
        match self {
            SelectOccurrence::First => 0b00,
            SelectOccurrence::Last => 0b01,
            SelectOccurrence::Next => 0b10,
            SelectOccurrence::Previous => 0b11,
        }
    }
}

// Encodes a SELECT command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Select<'a, RetT: Response> {
    pub file: &'a FileRef,
    pub occurrence: SelectOccurrence,

    _ret_t: std::marker::PhantomData<RetT>,
}

impl<'a, RetT: Response> Select<'a, RetT> {
    pub fn new(file: &'a FileRef) -> Self {
        Self {
            file,
            occurrence: SelectOccurrence::First,
            _ret_t: std::marker::PhantomData {},
        }
    }

    pub fn with_occurrence(mut self, occ: SelectOccurrence) -> Self {
        self.occurrence = occ;
        self
    }
    pub fn first(self) -> Self {
        self.with_occurrence(SelectOccurrence::First)
    }
    pub fn last(self) -> Self {
        self.with_occurrence(SelectOccurrence::Last)
    }
    pub fn next(self) -> Self {
        self.with_occurrence(SelectOccurrence::Next)
    }
    pub fn previous(self) -> Self {
        self.with_occurrence(SelectOccurrence::Previous)
    }

    fn p1(&self) -> u8 {
        match self.file {
            FileRef::Name(_) => 0b0100,
        }
    }
    fn p2(&self) -> u8 {
        self.occurrence.apdu_p2()
    }
}

impl<'a, RetT: Response> Request for Select<'a, RetT> {
    type Returns = RetT;

    fn ins(&self) -> u8 {
        0xA4
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (self.p1(), self.p2(), self.file.clone().into())
    }
}

#[cfg(test)]
mod tests {
    use crate::apdu;
    use crate::cmd::Request;
    use crate::refs::FileRef;

    #[test]
    fn test_select_emv_directory() {
        let aid = FileRef::Name("1PAY.SYS.DDF01".into());
        let sel = super::Select::<()>::new(&aid);
        assert_eq!(
            sel.to_apdu().unwrap(),
            apdu::Request::new(0x00, 0xA4, 0x04, 0x00, aid.to_vec()),
        );
    }
}
