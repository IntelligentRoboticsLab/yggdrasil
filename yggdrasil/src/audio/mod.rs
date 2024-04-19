use crate::prelude::*;

use self::audio_input::AudioInputModule;
use self::sound_manager::SoundManagerModule;
use self::whistle_detection::WhistleDetectionModule;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

pub mod audio_input;
pub mod sound_manager;
pub mod wee_sound;
pub mod whistle_detection;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AudioConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub wee_sound_timeout: Duration,

    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub whistle_timeout: Duration,
}

impl Config for AudioConfig {
    const PATH: &'static str = "audio.toml";
}

pub struct AudioModule;

impl Module for AudioModule {
    fn initialize(self, app: App) -> Result<App> {
        app.init_config::<AudioConfig>()?
            .add_module(SoundManagerModule)?
            .add_module(AudioInputModule)?
            .add_module(WhistleDetectionModule)
    }
}
