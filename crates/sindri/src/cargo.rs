use std::{
    ffi::OsStr, fmt::Debug, path::PathBuf, process::Stdio, result::Result, string::FromUtf8Error,
};

use miette::Diagnostic;

use thiserror::Error;
use tokio::process::Command;

/// Error kind that can occur when running cargo operations in sindri.
#[derive(Error, Diagnostic, Debug)]
pub enum CargoError {
    #[error(transparent)]
    #[diagnostic(help("Failed to spawn cargo child process!"))]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    CargoManifestError(#[from] cargo_toml::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to deserialize cargo unit-graph!"))]
    Json(#[from] serde_json::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to parse regex pattern!"))]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to parse stderr"))]
    FromUtf8(#[from] FromUtf8Error),

    #[error(
        "Cargo execution failed:
        {0}"
    )]
    #[diagnostic(help("Cargo output is printed above!"))]
    Execution(String),

    #[error("Invalid bin specified {0}")]
    #[diagnostic(help("Make sure you specify a valid bin, such as `yggdrasil`!"))]
    InvalidBin(String),
}

async fn cargo<I, S>(args: I) -> Result<(), CargoError>
where
    I: IntoIterator<Item = S> + Debug + Clone,
    S: AsRef<OsStr>,
{
    let output = Command::new("cargo")
        .args(args)
        .args(["--color", "always"]) // always pass color, cargo doesn't pass color when it detects it's piped
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    if !output.status.success() {
        // build failed for whatever reason, print to stdout
        let stderr = String::from_utf8(output.stderr)?;
        return Err(CargoError::Execution(stderr));
    }

    Ok(())
}

/// Perform a `cargo build` command using the specified arguments.
///
/// This spawns a cargo process using the provided properties as arguments.
///
/// If `target` is set to [`Option::None`], it will default to the current system's target.
pub async fn build(binary: &str, release: bool, target: Option<&str>) -> Result<(), CargoError> {
    let mut cargo_args = vec!["build", "-p", binary];

    if release {
        cargo_args.push("--release");
    }

    if let Some(target) = target {
        cargo_args.push("--target");
        cargo_args.push(target);
    }

    cargo(cargo_args).await
}

/// Assert that the provided bin is valid for the current cargo workspace.
///
/// This will result in an error if the command isn't executed in a cargo workspace, or if the provided bin isn't found.
pub fn assert_valid_bin(bin: &str) -> Result<(), CargoError> {
    let manifest =
        cargo_toml::Manifest::from_path("./Cargo.toml").map_err(CargoError::CargoManifestError)?;

    let Some(workspace) = manifest.workspace else {
        Err(CargoError::Execution(
            "The `--bin` flag has to be ran in a Cargo workspace.".to_owned(),
        ))?
    };

    for item in workspace.members.iter() {
        let path = PathBuf::from(item);

        if !path.exists() || !path.is_dir() {
            continue;
        }

        if path.ends_with(bin) {
            return Ok(());
        }
    }

    // We couldn't find it the bin!
    Err(CargoError::InvalidBin(bin.to_string()))?
}
