use self::{walking_engine_config::WalkingEngineConfig, yggdrasil_config::YggdrasilConfig};
use miette::Result;
use odal::ConfigResource;
use tyr::prelude::*;

pub mod walking_engine_config;
pub mod yggdrasil_config;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_config::<YggdrasilConfig>("../config/yggdrasil.toml")?
            .add_config::<WalkingEngineConfig>("../config/walking_engine.toml")
    }
}
