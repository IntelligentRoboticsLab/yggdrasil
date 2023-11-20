use clap::Parser;

pub mod deploy;
pub mod run;
pub mod scan;

/// `sindri` - The build tool for yggdrasil
///
/// `sindri` is a command-line interface tool designed for managing and deploying yggdrasil on robots.
/// It offers functionalities to scan for robots on the network, compile yggdrasil code, and deploy it to specific robots.
///
/// # Prerequisites
/// Ensure that your system is connected to the same network as the robots.
///
/// # Deploying Code to a Robot
/// To deploy yggdrasil to a specific robot:
/// ```sh
/// sindri deploy <robot-number>
/// ```
/// Replace `<robot-number>` with the actual number of the robot, found on the label on the robot's head.
///
/// # Scanning for Robots
/// To scan for available robots on the network:
/// ```sh
/// sindri scan
/// ```
/// This command scans for all robots specified in the current configuration.
///
/// ## Specifying a Range
/// You can limit the scan to a specific range of robot numbers:
/// ```sh
/// sindri scan --range 0 100
/// ```
/// This scans for robots numbered between 0 and 100.
///
/// # Additional Options
/// For more advanced options use `sindri --help`.

#[derive(Parser)]
#[clap(name = "sindri", version)]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging
    #[clap(short, long)]
    pub verbose: bool,

    /// Specify bin target
    #[clap(global = true, long, default_value = "yggdrasil")]
    pub bin: String,
}

/// All possible commands for the cli, used for clap derive macros.
#[derive(Parser)]
pub enum Commands {
    Deploy(deploy::Deploy),
    Run(run::Run),
    Scan(scan::Scan),
}
