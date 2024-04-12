//! Module for detecting the location of the ball in the field

pub mod proposal;

use proposal::BallProposalConfig;

use serde::{Deserialize, Serialize};

use crate::prelude::*;

pub struct BallDetectionModule;

impl Module for BallDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(proposal::BallProposalModule)?
            .init_config::<BallDetectionConfig>()?
            .add_startup_system(init_subconfigs)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallDetectionConfig {
    pub proposal: BallProposalConfig,
}

impl Config for BallDetectionConfig {
    const PATH: &'static str = "ball_detection.toml";
}

// TODO: find a better way to do this (reflection :sob:)
#[startup_system]
fn init_subconfigs(storage: &mut Storage, config: &mut BallDetectionConfig) -> Result<()> {
    storage.add_resource(Resource::new(config.proposal.clone()))?;

    Ok(())
}
