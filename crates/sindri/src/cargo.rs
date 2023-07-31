use std::{ffi::OsStr, fmt::Debug, process::Stdio, string::FromUtf8Error};

use miette::{Context, Diagnostic, Result};

use thiserror::Error;
use tokio::process::Command;

#[derive(Error, Diagnostic, Debug)]
enum CargoErrorKind {
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

async fn cargo<I, S>(args: I) -> Result<(), CargoErrorKind>
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
        return Err(CargoErrorKind::Cargo(stderr));
    }

    Ok(())
}

pub async fn build(binary: &str, release: bool, target: Option<&str>) -> Result<()> {
    let mut cargo_args = vec!["build", "-p"];
    cargo_args.push(binary);

    if release {
        cargo_args.push("--release");
    }

    if let Some(target) = target.as_ref() {
        cargo_args.push("--target");
        cargo_args.push(target);
    }

    cargo(cargo_args)
        .await
        .wrap_err("Failed to build yggdrasil!")
}
