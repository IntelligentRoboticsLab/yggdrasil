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
        match cargo::build(bin.clone(), self.build.release, None).await {
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
