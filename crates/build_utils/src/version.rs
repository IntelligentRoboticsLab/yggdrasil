use std::{
    fmt::{Display, Formatter},
    process::Command,
};

use colored::Colorize;
use miette::{miette, Result};

/// Information about the git repository where the crate was built.
#[derive(PartialEq, Debug)]
pub struct CommitInfo {
    pub short_commit_hash: String,
    pub commit_hash: String,
    pub commit_date: String,
}

/// Sindri's version.
#[derive(PartialEq)]
pub struct VersionInfo {
    /// The version of crate.
    pub version: String,

    /// Information about the git repository the crate may have been built from.
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
        format!("{value}").into()
    }
}

pub trait Version {
    const BIN_NAME: &'static str;
    const CRATE_PATH: &'static str;

    const PKG_VERSION: Option<&'static str>;
    const COMMIT_SHORT_HASH: Option<&'static str>;
    const COMMIT_HASH: Option<&'static str>;
    const COMMIT_DATE: Option<&'static str>;

    #[must_use]
    fn current() -> VersionInfo {
        let version = Self::PKG_VERSION.unwrap_or("0.0.0").to_string();

        let commit_info = match (
            Self::COMMIT_SHORT_HASH,
            Self::COMMIT_HASH,
            Self::COMMIT_DATE,
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
        let version = crate::cargo::find_bin_version(Self::BIN_NAME)?;
        // This command is executed in the `Self::CRATE_PATH`, and as such
        // we need to pass "." as last argument, to tell git that we
        // only care about changes in that directory.
        let output = match Command::new("git")
            .arg("log")
            .arg("-1")
            .arg("--date=short")
            .arg("--format=%H %h %cd")
            .arg(Self::CRATE_PATH)
            .output()
        {
            Ok(output) if output.status.success() => output,
            Ok(_) => {
                return Err(miette!(
                    "Non-zero process exit while obtaining commit hash for {}!",
                    Self::BIN_NAME,
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

    /// Checks if the current version of matches the current version in the workspace.
    /// If the versions differ, a message is printed to the console
    fn check_current_version() {
        let current_version = Self::current();
        let Ok(latest_version) = Self::find_latest() else {
            return;
        };

        if current_version == latest_version {
            return;
        }

        println!(
            "{}: {} {}",
            "warning".bold().yellow(),
            format!("newer {} version available:", Self::BIN_NAME).bold(),
            latest_version,
        );
        println!(
            " {} {}",
            "-->".dimmed().bold(),
            format!("run `{} update` to update", Self::BIN_NAME).dimmed()
        );
        println!();
    }
}
