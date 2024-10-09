use clap::Parser;
use miette::{IntoDiagnostic, Result};
use tokio::process::Command;

#[derive(Parser, Debug)]
/// Generate the dependency graph for Yggdrasil.
pub struct DependencyGraph;

impl DependencyGraph {
    pub async fn generate(&self) -> Result<()> {
        let working_dir = format!(
            "{}/deploy/",
            std::env::current_dir().into_diagnostic()?.display()
        );

        Command::new("cargo")
            .args(["run", "-r", "--features", "dependency_graph"])
            .current_dir(&working_dir)
            .kill_on_drop(true)
            .status()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
