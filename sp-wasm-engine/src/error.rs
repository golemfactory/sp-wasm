use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;
use std::path::StripPrefixError;
use std::string::FromUtf8Error;

use sp_wasm_memfs::error::Error as MemFSError;

use super::sandbox::engine::error::Error as EngineError;

#[derive(Debug)]
pub enum Error {
    InvalidPath,
    FileNotMapped,
    StripPrefix(StripPrefixError),
    FromUtf8(FromUtf8Error),
    MemFS(MemFSError),
    Io(IoError),
    Engine(EngineError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidPath => write!(f, "invalid path"),
            Error::FileNotMapped => write!(f, "file not mapped"),
            Error::StripPrefix(ref err) => err.fmt(f),
            Error::FromUtf8(ref err) => err.fmt(f),
            Error::MemFS(ref err) => err.fmt(f),
            Error::Io(ref err) => err.fmt(f),
            Error::Engine(ref err) => err.fmt(f),
        }
    }
}

impl StdError for Error {}

impl From<StripPrefixError> for Error {
    fn from(err: StripPrefixError) -> Error {
        Error::StripPrefix(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Error {
        Error::FromUtf8(err)
    }
}

impl From<MemFSError> for Error {
    fn from(err: MemFSError) -> Error {
        Error::MemFS(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl From<EngineError> for Error {
    fn from(err: EngineError) -> Error {
        Error::Engine(err)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::InvalidPath, &Error::InvalidPath) => true,
            (&Error::FileNotMapped, &Error::FileNotMapped) => true,
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
