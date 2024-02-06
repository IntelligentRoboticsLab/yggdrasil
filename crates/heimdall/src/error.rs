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

    /// Image error, this wraps a [image::ImageError]
    #[error(transparent)]
    Image(#[from] image::ImageError),

    #[error(transparent)]
    HorizontalFlip(io::Error),

    #[error(transparent)]
    VerticalFlip(io::Error),
}
