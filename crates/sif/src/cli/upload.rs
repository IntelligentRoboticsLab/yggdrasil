use clap::Parser;
use miette::Result;

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsUpload {
    pub robot_name: String, 
}

#[derive(Parser)]
#[clap(name = "upload")]
pub struct Upload {
    #[clap(flatten)]
    pub scan: ConfigOptsUpload,
}

impl Upload {
    /// Gets ip based on robot name and uploads to the robot
    pub async fn upload(self) -> Result<()> { Ok(()) }
}
