use clap::{Parser, ValueEnum};
use miette::{miette, IntoDiagnostic, Result};

#[derive(Debug, Clone, ValueEnum)]
enum PackagesToUpdate {
    Sindri,
    ReControl,
}

#[derive(Parser, Debug)]
pub struct UpdateCommand {
    /// Update the installation of a specific package.
    #[arg(required_unless_present = "all")]
    package: Option<PackagesToUpdate>,

    /// Update the installation of all packages.
    #[arg(short = 'a', long = "all", conflicts_with = "package")]
    all: bool,
}

impl UpdateCommand {
    pub async fn update(self) -> Result<()> {
        // Update all packages
        if self.all {
            return Self::update_all().await;
        }

        // Update a specific package
        if let Some(package) = self.package {
            match package {
                PackagesToUpdate::Sindri => Self::update_sindri().await?,
                PackagesToUpdate::ReControl => Self::update_re_control().await?,
            }
        }

        Ok(())
    }

    async fn update_sindri() -> Result<()> {
        Self::update_pkg("sindri", "tools/sindri").await
    }

    async fn update_re_control() -> Result<()> {
        Self::update_pkg("re_control", "tools/re_control").await
    }

    async fn update_all() -> Result<()> {
        Self::update_sindri().await?;
        Self::update_re_control().await?;

        Ok(())
    }

    async fn update_pkg(pkg_name: &str, pkg_path: &str) -> Result<()> {
        build_utils::cargo::find_bin_manifest(pkg_name)
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        tokio::process::Command::new("cargo")
            .args(["install", "--locked", "--path", pkg_path])
            .status()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
