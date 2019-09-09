use failure::_core::convert::TryFrom;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::path::Path;

pub struct Bytes(Vec<u8>);

impl Bytes {
    pub fn from_reader(mut input: impl Read) -> io::Result<Self> {
        let mut bytes = Vec::new();
        let data_read = input.read_to_end(&mut bytes)?;
        Ok(Bytes(bytes))
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(bytes: Vec<u8>) -> Self {
        Bytes(bytes)
    }
}

impl TryFrom<&Path> for Bytes {
    type Error = io::Error;

    fn try_from(value: &Path) -> io::Result<Self> {
        let mut file = OpenOptions::new().read(true).write(false).open(value)?;
        Bytes::from_reader(file)
    }
}
