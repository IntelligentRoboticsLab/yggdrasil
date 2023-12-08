use miette::Diagnostic;
use thiserror::Error;
use tokio::time::error::Elapsed;

/// Type alias for [`std::result::Result`] containing a sindri [`enum@Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Enum describing the possible errors that can occur in sindri.
#[derive(Error, Diagnostic, Debug)]
pub enum Error {
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    CargoError(crate::cargo::CargoError),

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
