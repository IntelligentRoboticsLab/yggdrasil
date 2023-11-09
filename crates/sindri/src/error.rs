use miette::Diagnostic;
use thiserror::Error;
use tokio::time::error::Elapsed;

pub type Result<T> = miette::Result<T, Error>;

#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    CargoManifestError(#[from] cargo_toml::Error),

    #[error("Cargo Error: {0}")]
    CargoError(String),

    #[error("Sftp error: {msg}")]
    SftpError {
        #[source]
        source: ssh2::Error,
        msg: String,
    },

    #[error("Failed to connect to robot in time: {0}")]
    #[diagnostic(
        code(connection::timeout),
        help(
            "- Is the robot powered on?
- Are you on the same network as the robot?"
        )
    )]
    ElapsedError(Elapsed),
}
