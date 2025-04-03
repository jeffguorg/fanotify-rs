#[macro_use]
mod macros;

pub mod consts;
pub mod error;
pub mod fanotify;

pub use bitflags;

pub use fanotify::{Error, Event, Fanotify, Response, Response as FanotifyResponse};
