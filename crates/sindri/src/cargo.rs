use std::{
    ffi::OsStr, fmt::Debug, path::Path, process::Stdio, result::Result, string::FromUtf8Error,
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
pub async fn build(
    binary: &str,
    profile: Profile,
    target: Option<&str>,
    features: Vec<&str>,
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

    // add required environment variables for cross compilation
    let mut envs = envs.unwrap_or_default();
    envs.extend_from_slice(cross::ENV_VARS);

    cargo(cargo_args, envs).await
}

/// Assert that the provided bin is valid for the current cargo workspace.
///
/// This will result in an error if the command isn't executed in a cargo workspace, or if the provided bin isn't found.
pub fn assert_valid_bin(bin: &str) -> Result<(), CargoError> {
    let manifest = cargo_toml::Manifest::from_path("./Cargo.toml").map_err(CargoError::Manifest)?;

    let Some(workspace) = manifest.workspace else {
        Err(CargoError::Workspace)?
    };

    for item in &workspace.members {
        let path = Path::new(item);

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

/// Environment variables that are required to cross compile for the robot, depending
/// on the current host architecture.
mod cross {
    #[cfg(target_os = "linux")]
    pub const ENV_VARS: &[(&str, &str)] = &[];

    #[cfg(target_os = "macos")]
    pub const ENV_VARS: &[(&str, &str)] = &[
        (
            "PKG_CONFIG_PATH",
            // homebrew directory is different for x86_64 and aarch64 macs!
            #[cfg(target_arch = "aarch64")]
            "/opt/homebrew/opt/x86_64-unknown-linux-gnu-alsa-lib/lib/x86_64-unknown-linux-gnu/pkgconfig",
            #[cfg(target_arch = "x86_64")]
            "/usr/local/opt/x86_64-unknown-linux-gnu-alsa-lib/lib/x86_64-unknown-linux-gnu/pkgconfig",
        ),
        ("PKG_CONFIG_ALLOW_CROSS", "1"),
        ("TARGET_CC", "x86_64-unknown-linux-gnu-gcc"),
        ("TARGET_CXX", "x86_64-unknown-linux-gnu-g++"),
        ("TARGET_AR", "x86_64-unknown-linux-gnu-ar"),
        (
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            "x86_64-unknown-linux-gnu-gcc",
        ),
    ];
}
