use crate::prelude::*;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

#[cfg(feature = "alsa")]
pub mod whistle_detection;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct HearingConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub whistle_timeout: Duration,
}

impl Config for HearingConfig {
    const PATH: &'static str = "hearing.toml";
}

pub struct HearingModule;

impl Module for HearingModule {
    fn initialize(self, app: App) -> Result<App> {
        #[cfg(feature = "alsa")]
        let app = app
            .init_config::<HearingConfig>()?
            .add_module(whistle_detection::WhistleDetectionModule)?;

        Ok(app)
    }
}
