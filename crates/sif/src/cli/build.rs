use clap::Parser;
use miette::Result;

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
        let mut cargo_args = vec!["build"];
        cargo_args.push("-p");
        cargo_args.push(&bin);

        if self.build.release {
            cargo_args.push("--release");
        }

        println!("building: {:?} release: {}", bin, self.build.release);
        cargo::cargo(cargo_args).await?;

        Ok(())
    }
}
