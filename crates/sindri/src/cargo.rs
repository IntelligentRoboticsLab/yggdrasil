use std::{ffi::OsStr, fmt::Debug, path::PathBuf, process::Stdio, string::FromUtf8Error};

use miette::{Context, Diagnostic, Result};

use thiserror::Error;
use tokio::process::Command;

use crate::error::Error;

#[derive(Error, Diagnostic, Debug)]
enum CargoError {
    #[error(transparent)]
    #[diagnostic(help("Failed to spawn cargo child process!"))]
    Io(#[from] std::io::Error),

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
    Cargo(String),
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
        return Err(CargoError::Cargo(stderr));
    }

    Ok(())
}

pub async fn build(binary: &str, release: bool, target: Option<&str>) -> Result<()> {
    let mut cargo_args = vec!["build", "-p", binary];

    if release {
        cargo_args.push("--release");
    }

    if let Some(target) = target {
        cargo_args.push("--target");
        cargo_args.push(target);
    }

    cargo(cargo_args)
        .await
        .wrap_err("Failed to build yggdrasil!")
}

/// Assert that the provided bin is valid for the current cargo workspace.
///
/// This will result in an error if the command isn't executed in a cargo workspace, or if the provided bin isn't found.
pub fn assert_valid_bin(bin: &str) -> Result<()> {
    let manifest =
        cargo_toml::Manifest::from_path("./Cargo.toml").map_err(Error::CargoManifestError)?;

    let Some(workspace) = manifest.workspace else {
        Err(Error::CargoError(
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

    // If the bin exists but we couldn't find it
    Err(Error::CargoError(
        "The specified bin does not exist.".to_string(),
    ))?
}
