pub mod error;
pub mod file;
pub mod memfs;
mod node;

pub type Result<T> = std::result::Result<T, error::Error>;

pub mod prelude {
    pub use super::file::File;
    pub use super::memfs::MemFS;
}

#[macro_use]
extern crate failure;
