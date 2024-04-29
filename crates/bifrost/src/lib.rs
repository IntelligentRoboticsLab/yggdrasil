//! Bifrost is a message protocol and library for robot soccer related communication for the Standard Platform League.
pub mod broadcast;
pub mod communication;
pub mod serialization;

mod error;
pub use error::{Error, Result};

extern crate self as bifrost;
