use std::io;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "file '{}' already exists", _0)]
    AlreadyExists(String),

    #[fail(display = "file '{}' not found", _0)]
    NotFound(String),

    #[fail(display = "invalid path: '{}'", _0)]
    InvalidPath(String),

    #[fail(display = "file is root")]
    IsRoot,

    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::AlreadyExists(ref left), &Error::AlreadyExists(ref right)) => left == right,
            (&Error::NotFound(ref left), &Error::NotFound(ref right)) => left == right,
            (&Error::InvalidPath(ref left), &Error::InvalidPath(ref right)) => left == right,
            (&Error::IsRoot, &Error::IsRoot) => true,
            (&Error::Io(ref left), &Error::Io(ref right)) => left.kind() == right.kind(),
            (_, _) => false,
        }
    }
}
