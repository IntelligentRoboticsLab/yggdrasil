//! Result and Error types for the crate.
use thiserror::Error;

/// Result containing an error variant from this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Communication error variants
#[derive(Error, Debug)]
pub enum Error {
    /// IO error, this wraps a [std::io::Error]
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}