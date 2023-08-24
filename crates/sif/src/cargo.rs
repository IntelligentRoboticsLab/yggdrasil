use std::{ffi::OsStr, fmt::Debug, process::Stdio, string::FromUtf8Error};

use miette::{Context, Diagnostic, Result};
use tokio::process::Command;
use thiserror::Error;


#[derive(Error, Diagnostic, Debug)]
enum CargoErrorKind {
    #[error(transparent)]
    #[diagnostic(help("Failed to spawn cargo child process!"))]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    #[diagnostic(help("Failed to parse stderr"))]
    FromUtf8Error(#[from] FromUtf8Error),

    #[error(
        "Cargo execution failed:
        {0}"
    )]
    #[diagnostic(help("Cargo output is printed above!"))]
    CargoError(String),
}

async fn cargo<I, S>(args: I) -> Result<(), CargoErrorKind>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr>,
{
    let output = Command::new("cargo")
        .args(args)
        .arg("--color") // always pass color, cargo doesn't pass color when it detects it's piped
        .arg("always")
        .stderr(Stdio::null())
        .output()
        .await
        .map_err(CargoErrorKind::IoError)?;

    if !output.status.success() {
        let stderr = String::from_utf8(output.stderr).map_err(CargoErrorKind::FromUtf8Error)?;
        // eprintln!("{stderr}");
        Err(CargoErrorKind::CargoError(
            stderr
                .clone()
        ))?;
    }

    Ok(())
}

pub async fn build(binary: String, release: bool, target: Option<String>) -> Result<()> {
    let mut cargo_args = vec!["build", "-p"];
    cargo_args.push(&binary);

    if release {
        cargo_args.push("--release");
    }

    if let Some(target) = target.as_ref() {
        cargo_args.push("--target");
        cargo_args.push(target.as_str());
    }

    cargo(cargo_args)
        .await
        .wrap_err("Failed to build yggdrasil!")
}