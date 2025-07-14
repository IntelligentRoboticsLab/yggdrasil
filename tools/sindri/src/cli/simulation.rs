use clap::Parser;
use miette::{Result, IntoDiagnostic};
use std::process::Stdio;

/// Run the simulation locally using the simulation binary.
///
/// This command runs `cargo run --bin simulation -r` in the `/deploy` folder at the root of the project.
#[derive(Parser, Debug)]
#[clap(about = "Run the simulation locally using the simulation binary")] 
pub struct Simulation {}

impl Simulation {
    pub async fn run(self) -> Result<()> {
        let deploy_dir = std::path::Path::new("deploy");
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.arg("run").arg("--bin").arg("simulation").arg("-r");
        cmd.current_dir(deploy_dir);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let mut child = cmd.spawn().into_diagnostic()?;
        let status = child.wait().await.into_diagnostic()?;
        if !status.success() {
            Err(miette::miette!("Simulation process exited with an error"))
        } else {
            Ok(())
        }
    }
}
