#![warn(missing_docs)]
//! Contains functionality for the Heimdall camera module
pub mod camera;

mod error;
pub use error::Result;

/// The camera width of a NAO v6.
pub const NAO_V6_CAMERA_WIDTH: u32 = 1280;

/// The camera height of a NAO v6.
pub const NAO_V6_CAMERA_HEIGHT: u32 = 960;
