use clap::Parser;
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
        match cargo::build(bin, self.build.release, None).await {
            Ok(_) => {
                println!("done!");
            }
            Err(err) => {
                return Err(err)?;
            }
        }

        Ok(())
    }
}
