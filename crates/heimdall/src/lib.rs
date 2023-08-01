#![warn(missing_docs)]
#![feature(portable_simd)]

//! Contains functionality for the Heimdall camera module
mod camera;
pub use camera::Camera;

/// Contains functionality for retrieving additional camera information.
pub mod device;

mod error;
pub use error::Result;
