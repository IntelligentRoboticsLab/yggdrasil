use crate::{
    cargo::{self, Profile},
    config::Config,
    cli::deploy::deploy_to_robot
};
use clap::Parser;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::time::Duration;
use std::fs;
use colored::Colorize;
use miette::{miette, Context, IntoDiagnostic};


const ROBOT_TARGET: &str = "x86_64-unknown-linux-gnu";
const RELEASE_PATH: &str = "./target/x86_64-unknown-linux-gnu/release/skadi";
const DEPLOY_PATH: &str = "./deploy/skadi";


#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsRecord {
    /// Number of the robot to deploy to.
    #[clap(index = 1, name = "robot-number")]
    pub number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long)]
    pub wired: bool,

    /// Team number [default: Set in `sindri.toml`]
    #[clap(short, long)]
    pub team_number: Option<u8>,
}

impl ConfigOptsRecord {
    #[must_use]
    pub fn new(number: u8, wired: bool, team_number: Option<u8>) -> Self {
        Self {
            number,
            wired,
            team_number,
        }
    }
}


/// Utility for recording motions
#[derive(Parser)]
pub struct Record {
    #[clap(flatten)]
    pub record: ConfigOptsRecord,
}

/* This procedure is a very similar procedure to sindri deploy, only instead of yggdrasil getting pushed skadi gets compiled and pushed to the robot.
*/

impl Record {
    pub async fn record(self, config: Config) -> miette::Result<()> {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template(
                "   {prefix:.green.bold} skadi {msg} {spinner:.green.bold}",
            )
            .unwrap()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );

        pb.set_message(format!(
            "{}{}, {}{}{}",
            "(release: ".dimmed(),
            "true".red(),
            "target: ".dimmed(),
            ROBOT_TARGET.bold(),
            ")".dimmed()
        ));
        pb.set_prefix("Compiling");

        // Build yggdrasil with cargo
        cargo::build("skadi", Profile::Release, Some(ROBOT_TARGET), &Vec::new(), None).await?;

        pb.println(format!(
            "{} {} {}{}, {}{}{}",
            "   Compiling".green().bold(),
            "skadi".bold(),
            "(release: ".dimmed(),
            "true".red(),
            "target: ".dimmed(),
            ROBOT_TARGET.bold(),
            ")".dimmed()
        ));

        pb.println(format!(
            "{} in {}",
            "    Finished".green().bold(),
            HumanDuration(pb.elapsed()),
        ));
        pb.reset_elapsed();

        // Check if the robot exists
        let robot = config
            .robot(self.record.number, self.record.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.record.number
            )))?;

        pb.set_style(
            ProgressStyle::with_template("   {prefix:.blue.bold} {msg} {spinner:.blue.bold}")
                .unwrap()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        );

        pb.set_prefix("Deploying");
        pb.set_message(format!("{}", "Preparing deployment...".dimmed()));

        // Copy over the files that need to be deployed
        fs::copy(RELEASE_PATH, DEPLOY_PATH)
            .into_diagnostic()
            .wrap_err("Failed to copy binary to deploy directory!")?;

        deploy_to_robot(&pb, robot.ip())
            .await
            .wrap_err("Failed to deploy yggdrasil files to robot")?;

        pb.println(format!(
            "{} in {}",
            "  Deployed to robot".bold(),
            HumanDuration(pb.elapsed()),
        ));
        pb.finish_and_clear();

        robot
            .ssh("./skadi", Vec::<(&str, &str)>::new())?
            .wait()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}