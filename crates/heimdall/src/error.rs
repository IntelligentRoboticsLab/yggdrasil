//! Result and Error types for the crate.
use std::io;

use miette::Diagnostic;
use thiserror::Error;

/// Result containing an error variant from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Camera error variants.
#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    /// IO error, this wraps a [std::io::Error]
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error("Failed to open camera device at `{path}`")]
    DeviceOpen {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to set camera property `{property}` to `{value}`")]
    DeviceProperty {
        property: String,
        value: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to set the device to video capture mode")]
    VideoCapture(#[source] io::Error),

    /// Image error, this wraps a [image::ImageError]
    #[error(transparent)]
    Image(#[from] image::ImageError),

    #[error("Failed to flip camera horizontally")]
    HorizontalFlip(#[source] io::Error),

    #[error("Failed to flip camera vertically")]
    VerticalFlip(#[source] io::Error),

    #[error("Failed to set exposure weights")]
    SetAutoExposureWeights(io::Error),

    #[error(transparent)]
    Jpeg(turbojpeg::Error),
}
