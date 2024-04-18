use miette::Diagnostic;
use thiserror::Error;
use tokio::time::error::Elapsed;

/// Type alias for [`std::result::Result`] containing a sindri [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Enum describing the possible errors that can occur in sindri.
#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Cargo(crate::cargo::CargoError),

    #[error("Sftp error: {msg}")]
    Sftp {
        #[source]
        source: ssh2::Error,
        msg: String,
    },
    #[error("Ssh error: {command}")]
    Ssh {
        #[source]
        source: std::io::Error,
        command: String,
    },
    #[error("Failed to connect to robot in time: {0}")]
    #[diagnostic(
        code(connection::timeout),
        help(
            "- Is the robot powered on?
- Are you on the same network as the robot?"
        )
    )]
    Elapsed(Elapsed),
}
