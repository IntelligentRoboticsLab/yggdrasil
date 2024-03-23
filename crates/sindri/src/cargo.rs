use std::{ffi::OsStr, fmt::Debug, result::Result, string::FromUtf8Error};

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
    Manifest(#[from] cargo_toml::Error),

    #[error("The --bin flag has to be used in a Cargo workspace.")]
    #[diagnostic(help("Make sure you run sindri in the root of yggdrasil!"))]
    Workspace,

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

/// Possible profiles used when building.
#[derive(Debug, Clone, Copy)]
pub enum Profile {
    Debug,
    Release,
}

async fn cargo<I, E, S>(args: I, envs: E) -> Result<(), CargoError>
where
    I: IntoIterator<Item = S> + Debug + Clone,
    E: IntoIterator<Item = (S, S)> + Debug + Clone,
    S: AsRef<OsStr>,
{
    let output = Command::new("cargo")
        .args(args)
        .args(["--color", "always"]) // always pass color, cargo doesn't pass color when it detects it's piped
        .envs(envs)
        .output()
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
pub async fn build(
    binary: &str,
    profile: Profile,
    target: Option<&str>,
    features: &[&str],
    envs: Option<Vec<(&str, &str)>>,
) -> Result<(), CargoError> {
    let mut cargo_args = vec!["build", "-p", binary];

    if matches!(profile, Profile::Release) {
        cargo_args.push("--release");
    }

    if let Some(target) = target {
        cargo_args.push("--target");
        cargo_args.push(target);
    }

    let feature_string = features.join(",");
    if !features.is_empty() {
        cargo_args.push("--features");
        cargo_args.push(feature_string.as_str());
    }

    cargo(cargo_args, envs.unwrap_or_default()).await
}

pub fn find_bin_manifest(bin: &str) -> Result<cargo_toml::Manifest, CargoError> {
    cargo_toml::Manifest::from_path("./Cargo.toml")
        .map_err(CargoError::Manifest)?
        .workspace
        .iter()
        .flat_map(|workspace| &workspace.members)
        .flat_map(|member| glob::glob_with(member, glob::MatchOptions::new()))
        .flatten()
        .flatten()
        .map(|mut member| {
            member.push("Cargo.toml");
            member
        })
        .filter(|path| path.exists() && path.is_file())
        .flat_map(|path| cargo_toml::Manifest::from_path(path).map_err(CargoError::Manifest))
        .find_map(|manifest| {
            manifest
                .bin
                .iter()
                .filter_map(|product| product.name.clone())
                .find(|name| name == bin)
                .map(|_| manifest)
        })
        .ok_or_else(|| CargoError::InvalidBin(bin.to_string()))
}

pub fn find_bin_version(bin: &str) -> Result<String, CargoError> {
    find_bin_manifest(bin).map(|manifest| manifest.package().version().to_string())
}
