#![warn(clippy::all)]

#[macro_use]
extern crate mozjs;
extern crate libc;

pub mod sandbox;

pub mod prelude {
    pub use crate::sandbox::engine::Engine;
    pub use crate::sandbox::Sandbox;
}
