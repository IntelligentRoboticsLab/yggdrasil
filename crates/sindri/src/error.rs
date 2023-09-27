use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    CargoManifestError(#[from] cargo_toml::Error),

    #[error("Cargo Error: {0}")]
    CargoError(String),
}
