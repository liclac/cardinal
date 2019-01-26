use crate::core::{File, Request};

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
pub struct Select<'a> {
    pub file: &'a File,
    pub occurrence: SelectOccurrence,
}

impl<'a> Select<'a> {
    pub fn new(file: &'a File) -> Self {
        Self {
            file,
            occurrence: SelectOccurrence::First,
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
            File::EF(_) | File::DF(_) | File::MF => 0b0000,
            File::AID(_) => 0b0100,
        }
    }
    fn p2(&self) -> u8 {
        self.occurrence.apdu_p2()
    }
}

impl<'a> Request for Select<'a> {
    type Returns = ();

    fn ins(&self) -> u8 {
        0xA4
    }
    fn data(&self) -> (u8, u8, Vec<u8>) {
        (self.p1(), self.p2(), self.file.clone().into())
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{apdu, File, Request};

    #[test]
    fn test_select_cirrus() {
        let aid = File::AID(vec![0xA0, 0x00, 0x00, 0x00, 0x04, 0x60, 0x00]);
        let sel = super::Select::new(&aid);
        assert_eq!(
            sel.to_apdu().unwrap(),
            apdu::Request::new(0x00, 0xA4, 0x04, 0x00, aid.to_vec()),
        );
    }
}
