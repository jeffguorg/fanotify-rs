#[macro_use]
mod macros;

pub mod error;
pub mod fanotify;
pub mod consts;

pub use bitflags;

pub use fanotify::{Fanotify, Error, Response, Response as FanotifyResponse, Event};