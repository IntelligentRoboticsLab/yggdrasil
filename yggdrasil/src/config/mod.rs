use miette::Result;
use odal::Configuration;
use tyr::prelude::*;

use self::yggdrasil_config::MainConfig;
use self::walking_engine_config::WalkingEngineConfig;

pub mod yggdrasil_config;
pub mod walking_engine_config;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> Result<App> {
        app
            .add_resource(Resource::new(MainConfig::load("../config/overlays/daphne/yggdrasil.toml")))?
            .add_resource(Resource::new(WalkingEngineConfig::load("../config/overlays/daphne/walking_engine.toml")))
    }
}