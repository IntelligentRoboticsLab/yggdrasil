use crate::prelude::*;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

pub mod audio_input;
pub mod battery_sound;
pub mod sound_manager;
pub mod wee_sound;
pub mod whistle_detection;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Resource)]
#[serde(deny_unknown_fields)]
pub struct AudioConfig {
    /// How long consecutive ground contact needs to be not detected before the robot goes from
    /// "grounded" to "ungrounded" state., in milliseconds
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub wee_sound_ungrounded_timeout: Duration,

    /// How long consecutive ground contact needs to be detected before the robot goes from
    /// "ungrounded" to "grounded" state, in milliseconds.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub wee_sound_grounded_timeout: Duration,

    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub battery_sound_timeout: Duration,

    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub whistle_timeout: Duration,
}

impl Config for AudioConfig {
    const PATH: &'static str = "audio.toml";
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<AudioConfig>().add_plugins((
            audio_input::AudioInputPlugin,
            sound_manager::SoundManagerPlugin,
            whistle_detection::WhistleDetectionPlugin,
        ));
    }
}
