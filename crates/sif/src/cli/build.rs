use clap::Parser;
use miette::Result;
use spinoff::{Color, Spinner};

use crate::cargo;

/// Config options for the build system
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsBuild {
    /// Build in release mode [default: false]
    #[clap(long)]
    pub release: bool,
}

#[derive(Parser)]
#[clap(name = "build")]
pub struct Build {
    #[clap(flatten)]
    pub build: ConfigOptsBuild,
}

impl Build {
    pub async fn build(self, bin: String) -> Result<()> {
        let spinner = Spinner::new(
            spinoff::spinners::Aesthetic,
            "Building yggdrasil",
            Color::Green,
        );

        match cargo::build(bin.clone(), self.build.release, None).await {
            Ok(_) => {
                spinner.success("Finished building yggdrasil! 🌳");
            }
            Err(err) => {
                spinner.stop_and_persist("X", "FUCKING D");
                return Err(err)?;
            }
        }

        Ok(())
    }
}
