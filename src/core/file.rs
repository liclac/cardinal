// Describes a reference to a file, either an EF's filename, a DF's AID, or the
// MF (Master File/Root).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum File {
    EF(Vec<u8>), // Elementary Files.
    DF(Vec<u8>), // Dedicated Files.
    AID(Vec<u8>), // DF AID; potentially truncated.
    MF, // Master File, aka root.
}

impl File {
    pub fn id(&self) -> &[u8] {
        match self {
            File::EF(id) => id.as_slice(),
            File::DF(id) => id.as_slice(),
            File::AID(id) => id.as_slice(),
            File::MF => &[0x3F, 0x00],
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.clone().into()
    }
}

impl Into<Vec<u8>> for File {
    fn into(self) -> Vec<u8> {
        match self {
            File::EF(id) => id,
            File::DF(id) => id,
            File::AID(id) => id,
            File::MF => vec![0x3F, 0x00],
        }
    }
}
