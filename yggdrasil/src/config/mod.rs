use self::yggdrasil_config::YggdrasilConfig;
use miette::Result;
use odal::ConfigResource;
use tyr::prelude::*;

mod yggdrasil_config;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_config::<YggdrasilConfig>("../config/yggdrasil.toml")
    }
}
