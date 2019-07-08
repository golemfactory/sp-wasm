use super::sandbox::engine::error::Error as EngineError;
use sp_wasm_memfs::error::Error as MemFSError;
use std::io::Error as IoError;
use std::path::StripPrefixError;
use std::string::FromUtf8Error;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "invalid path: {}", _0)]
    InvalidPath(String),

    #[fail(display = "{}", _0)]
    StripPrefix(#[cause] StripPrefixError),

    #[fail(display = "{}", _0)]
    FromUtf8(#[cause] FromUtf8Error),

    #[fail(display = "{}", _0)]
    MemFS(#[cause] MemFSError),

    #[fail(display = "{}", _0)]
    Io(#[cause] IoError),

    #[fail(display = "{}", _0)]
    Engine(#[cause] EngineError),
}

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
