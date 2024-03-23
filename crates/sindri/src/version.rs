use std::{
    fmt::{Display, Formatter},
    process::Command,
};

use colored::Colorize;
use miette::{miette, Result};

/// Checks if the current version of matches the current version in the workspace.
/// If the versions differ, a message is printed to the console
pub fn check_current_version() {
    let sindri_version = VersionInfo::current();
    let Ok(latest_version) = VersionInfo::find_latest() else {
        return;
    };
    if sindri_version == latest_version {
        return;
    }

    println!(
        "{}: {} {}",
        "warning".bold().yellow(),
        "newer sindri version available:".bold(),
        latest_version,
    );
    println!(
        " {} {}",
        "-->".dimmed().bold(),
        "run `sindri update` to update".dimmed()
    );
    println!();
}

/// Information about the git repository where sindri was built.
#[derive(PartialEq)]
pub struct CommitInfo {
    pub short_commit_hash: String,
    pub commit_hash: String,
    pub commit_date: String,
}

/// Sindri's version.
#[derive(PartialEq)]
pub struct VersionInfo {
    /// The version of sindri.
    pub version: String,

    /// Information about the git repository sindri may have been built from.
    ///
    /// `None` if not built from a git repo.
    pub commit_info: Option<CommitInfo>,
}

impl Display for VersionInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{}", self.version)?;

        if let Some(ci) = &self.commit_info {
            write!(f, " ({} {})", ci.short_commit_hash, ci.commit_date)?;
        };
        Ok(())
    }
}

impl From<VersionInfo> for clap::builder::Str {
    fn from(value: VersionInfo) -> Self {
        format!("{}", value).into()
    }
}

impl VersionInfo {
    pub fn current() -> VersionInfo {
        let version = option_env!("CARGO_PKG_VERSION")
            .unwrap_or("0.0.0")
            .to_string();

        let commit_info = match (
            option_env!("SINDRI_COMMIT_SHORT_HASH"),
            option_env!("SINDRI_COMMIT_HASH"),
            option_env!("SINDRI_COMMIT_DATE"),
        ) {
            (Some(short_commit_hash), Some(commit_hash), Some(commit_date)) => Some(CommitInfo {
                short_commit_hash: short_commit_hash.to_string(),
                commit_hash: commit_hash.to_string(),
                commit_date: commit_date.to_string(),
            }),
            _ => None,
        };

        VersionInfo {
            version,
            commit_info,
        }
    }

    fn find_latest() -> Result<VersionInfo> {
        let version = crate::cargo::find_bin_version("sindri")?;
        // This command is executed in "crates/sindri", and as such
        // we need to pass "." as last argument, to tell git that we
        // only care about changes in that directory.
        let output = match Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--date=short")
            .arg("--format=%H %h %cd")
            .arg("crates/sindri")
            .output()
        {
            Ok(output) if output.status.success() => output,
            Ok(_) => {
                return Err(miette!(
                    "Non-zero process exit while obtaining commit hash for sindri!"
                ));
            }
            Err(e) => {
                eprintln!();
                return Err(miette!("version: Failed to spawn git process: {}", e));
            }
        };

        let stdout = String::from_utf8(output.stdout).unwrap();
        let mut parts = stdout.split_whitespace();
        let commit_info = match (parts.next(), parts.next(), parts.next()) {
            (Some(commit_hash), Some(short_commit_hash), Some(commit_date)) => Some(CommitInfo {
                short_commit_hash: short_commit_hash.to_string(),
                commit_hash: commit_hash.to_string(),
                commit_date: commit_date.to_string(),
            }),
            _ => None,
        };

        Ok(VersionInfo {
            version,
            commit_info,
        })
    }
}
