use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file '{0}' already exists")]
    AlreadyExists(String),
    #[error("file '{0}' not found")]
    NotFound(String),
    #[error("invalid path: '{0}'")]
    InvalidPath(String),
    #[error("file is root")]
    IsRoot,
    #[error("{0}")]
    Io(#[from] io::Error),
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
