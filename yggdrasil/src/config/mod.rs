use miette::Result;
use tyr::prelude::*;

pub mod example_config;
pub mod walking_engine_config;
pub mod yggdrasil_config;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> Result<App> {
        // Add the loaded configurations.
        Ok(app)
    }
}
