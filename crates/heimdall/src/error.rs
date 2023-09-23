//! Result and Error types for the crate.
use thiserror::Error;

/// Result containing an error variant from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Communication error variants
#[derive(Error, Debug)]
pub enum Error {
    /// IO error, this wraps a [std::io::Error]
    #[error(transparent)]
    IO(#[from] std::io::Error),

    /// Camera error, this wraps a [rscam::Error]
    #[error(transparent)]
    Camera(#[from] rscam::Error),

    /// Image error, this wraps a [image::ImageError]
    #[error(transparent)]
    Image(#[from] image::ImageError),
}
