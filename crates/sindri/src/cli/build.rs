use std::time::Duration;

use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use miette::Result;

use crate::cargo;

/// Config options for the build system
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsBuild {
    /// Build in release mode
    #[clap(long, short)]
    pub release: bool,
}

#[derive(Parser)]
#[clap(name = "build")]
pub struct Build {
    #[clap(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub async fn build(self, bin: &str) -> Result<()> {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::with_template(
                "  {prefix:.green.bold} yggdrasil {msg} {spinner:.green.bold}",
            )
            .unwrap()
            .tick_chars("⠋⠙⠚⠞⠖⠦⠴⠲⠳⠓ "),
        );

        pb.set_message(format!(
            "{}{}, {}{}",
            "(release: ".dimmed(),
            self.build.release.to_string().red(),
            "target: ".dimmed(),
            "default)".dimmed()
        ));
        pb.set_prefix("Building");

        cargo::build(bin, self.build.release, None)
            .await
            .map_err(|e| {
                pb.abandon();
                e
            })?;
        pb.finish_and_clear();
        println!(
            "   {} in {}",
            "Finished".green().bold(),
            HumanDuration(pb.elapsed())
        );
        Ok(())
    }
}
