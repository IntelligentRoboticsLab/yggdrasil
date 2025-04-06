use std::{path::PathBuf, process::Stdio, time::Duration};

use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use miette::{IntoDiagnostic, Result};
use tokio::process::{Child, Command};

use crate::{
    cli::robot_ops,
    config::{Robot, SindriConfig},
};

use super::robot_ops::NameOrNum;

/// Flashes a new `.opn` image to the specified robot
#[derive(Parser, Debug)]
pub struct Flash {
    /// Scan for wired (true) or wireless (false) robots
    #[clap(long, short)]
    pub wired: bool,
    /// The robot ids to flash
    #[clap(required = true)]
    pub robot_ids: Vec<NameOrNum>,
    /// The path to the image to flash
    pub image: PathBuf,
}

impl Flash {
    pub async fn flash(self, config: SindriConfig) -> Result<()> {
        assert!(self.image.exists(), "Image file does not exist!");
        assert!(self.image.is_file(), "Image path is not a file!");

        let mut robots = self
            .robot_ids
            .iter()
            .map(|robot_id| {
                config.robot(robot_id, self.wired).unwrap_or_else(|| {
                    panic!("Invalid robot specified, robot {robot_id} is not configured!",)
                })
            })
            .collect::<Vec<_>>();

        robots.sort_by_key(|robot| robot.number);

        let multi = MultiProgress::new();
        multi.set_alignment(indicatif::MultiProgressAlignment::Bottom);
        let deploy_bar = multi.add(
            ProgressBar::new_spinner().with_style(
                ProgressStyle::with_template(
                    "   {prefix:.blue.bold} to robots {msg} {spinner:.blue.bold}",
                )
                .unwrap(),
            ),
        );
        deploy_bar.enable_steady_tick(Duration::from_millis(80));
        deploy_bar.set_prefix(" Flashing");
        deploy_bar.set_message(format!(
            "{}{}{}",
            "(robots: ".dimmed(),
            self.robot_ids.len().to_string().bold(),
            ")".dimmed()
        ));

        let mut join_set = tokio::task::JoinSet::new();
        for robot in robots {
            let multi = multi.clone();

            join_set.spawn({
                let image = self.image.clone();

                async move {
                    let multi = multi.clone();
                    let pb = ProgressBar::new_spinner();
                    let pb = multi.add(pb);
                    let output = robot_ops::Output::Multi(pb);

                    output.flashing_upload_phase(&image, &robot);

                    // ensure directory exists
                    robot
                        .ssh::<&str, &str>("sudo mkdir -p /home/.image", [], true)
                        .expect("Failed to create directory")
                        .wait()
                        .await?;

                    // rsync the image to the robot
                    spawn_image_rsync(image, &robot)
                        .expect("Failed to spawn rsync")
                        .wait()
                        .await
                        .expect("Failed to upload image");

                    output.finished_flashing(&robot.ip());

                    // reboot the robot
                    robot
                        .ssh::<&str, &str>("sudo shutdown -r now", [], true)
                        .expect("Failed to reboot robot")
                        .wait()
                        .await
                }
            });
        }

        join_set
            .join_all()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()?;

        deploy_bar.finish();
        println!(
            "{} in {}",
            "    Finished".cyan().bold(),
            HumanDuration(deploy_bar.elapsed()),
        );

        Ok(())
    }
}

fn spawn_image_rsync(image: PathBuf, robot: &Robot) -> Result<Child, std::io::Error> {
    Command::new("rsync")
        .arg("--rsync-path")
        .arg("sudo rsync")
        .arg(image.canonicalize().unwrap())
        .arg(format!("nao@{}:/home/.image/", robot.ip()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}
