use super::sandbox::engine::error::Error as EngineError;
use sp_wasm_memfs::error::Error as MemFSError;
use std::io::Error as IoError;
use std::path::StripPrefixError;
use std::string::FromUtf8Error;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("{0}")]
    StripPrefix(#[from] StripPrefixError),
    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),
    #[error("{0}")]
    MemFS(#[from] MemFSError),
    #[error("{0}")]
    Io(#[from] IoError),
    #[error("{0}")]
    Engine(#[from] EngineError),
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::InvalidPath(ref p_left), &Error::InvalidPath(ref p_right)) => {
                p_left == p_right
            }
            (&Error::StripPrefix(ref left), &Error::StripPrefix(ref right)) => left == right,
            (&Error::FromUtf8(ref left), &Error::FromUtf8(ref right)) => {
                left.utf8_error() == right.utf8_error()
            }
            (&Error::MemFS(ref left), &Error::MemFS(ref right)) => left == right,
            (&Error::Io(ref left), &Error::Io(ref right)) => left.kind() == right.kind(),
            (&Error::Engine(ref left), &Error::Engine(ref right)) => left == right,
            (_, _) => false,
        }
    }
}
