// Describes a reference to a file ID, either an EF's filename, a DF's AID, or the
// MF (Master File/Root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileID {
    EF(Vec<u8>),  // Elementary Files.
    DF(Vec<u8>),  // Dedicated Files.
    AID(Vec<u8>), // DF AID; potentially truncated.
    MF,           // Master File, aka root.
}

impl FileID {
    pub fn id(&self) -> &[u8] {
        match self {
            FileID::EF(id) => id.as_slice(),
            FileID::DF(id) => id.as_slice(),
            FileID::AID(id) => id.as_slice(),
            FileID::MF => &[0x3F, 0x00],
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.clone().into()
    }
}

impl Into<Vec<u8>> for FileID {
    fn into(self) -> Vec<u8> {
        match self {
            FileID::EF(id) => id,
            FileID::DF(id) => id,
            FileID::AID(id) => id,
            FileID::MF => vec![0x3F, 0x00],
        }
    }
}
