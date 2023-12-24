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

    /// VarInt too large, this occurs when the data being decoded
    /// is too large to fit into a 32-bit integer.
    #[error("VarInt too large")]
    VarIntError,

    /// Invalid string, this can occur while decoding a string
    #[error(transparent)]
    InvalidStringError(#[from] std::string::FromUtf8Error),

    /// Invalid Variant Id, this occurs while decoding an Enum
    /// that is encoded with a variant discriminant that's not known.
    #[error("Got an invalid variant discriminant ({0}) in enum: {1}")]
    InvalidVariantDiscriminant(usize, &'static str),
}
