use miette::Result;
use tyr::prelude::*;

pub mod walking_engine_config;
pub mod yggdrasil_config;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> Result<App> {
        // Add the configs here as resources (for this we need to know which robot the code runs on)
        //
        // Example:
        //
        // app.add_resource(Resource::new(MainConfig::load(
        //    "../config/overlays/daphne/yggdrasil.toml",
        // )))
        Ok(app)
    }
}
