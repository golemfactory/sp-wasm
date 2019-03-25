use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;

#[derive(Debug)]
pub enum Error {
    AlreadyExists,
    NotFound,
    InvalidPath,
    IsRoot,
    Io(IoError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::AlreadyExists => write!(f, "File already exists"),
            Error::NotFound => write!(f, "File not found"),
            Error::InvalidPath => write!(f, "Invalid path"),
            Error::IsRoot => write!(f, "File is root"),
            Error::Io(ref err) => err.fmt(f),
        }
    }
}

impl StdError for Error {}

impl From<IoError> for Error {
    fn from(err: IoError) -> Error {
        Error::Io(err)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::AlreadyExists, &Error::AlreadyExists) => true,
            (&Error::NotFound, &Error::NotFound) => true,
            (&Error::InvalidPath, &Error::InvalidPath) => true,
            (&Error::IsRoot, &Error::IsRoot) => true,
            (&Error::Io(ref left), &Error::Io(ref right)) => left.kind() == right.kind(),
            (_, _) => false,
        }
    }
}
