#[macro_use]
extern crate mozjs;

pub mod error;
pub mod sandbox;

pub use error::{Error, Result};

pub mod prelude {
    pub use super::sandbox::engine::{Engine, Runtime};
    pub use super::sandbox::vfs::VirtualFS;
    pub use super::sandbox::Sandbox;
}
