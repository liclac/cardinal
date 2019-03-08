// Describes a reference to a file ID, either an EF's filename, a DF's AID, or the
// MF (Master File/Root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRef {
    Name(Vec<u8>), // File name, eg. '1PAY.SYS.DDF01'.
}

impl FileRef {
    pub fn id(&self) -> &[u8] {
        match self {
            FileRef::Name(id) => id.as_slice(),
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.clone().into()
    }
}

impl Into<Vec<u8>> for FileRef {
    fn into(self) -> Vec<u8> {
        match self {
            FileRef::Name(id) => id,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RecordRef {
    Number { sfi: u8, num: u8 },
}

impl RecordRef {
    pub fn num(sfi: u8, num: u8) -> RecordRef {
        RecordRef::Number { sfi, num }
    }
}
