use crate::prelude::*;

use self::{button::ButtonFilter, fsr::FSRFilter, imu::IMUFilter, sonar::SonarFilter};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

#[cfg(feature = "alsa")]
pub mod audio_input;
pub mod button;
pub mod fsr;
pub mod imu;
pub mod sonar;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FilterConfig {
    pub button_activation_threshold: f32,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub button_held_duration_threshold: Duration,
    pub ground_contact_threshold: f32,
}

pub struct FilterModule;

impl Module for FilterModule {
    fn initialize(self, app: App) -> Result<App> {
        let app = app
            .add_module(ButtonFilter)?
            .add_module(FSRFilter)?
            .add_module(IMUFilter)?
            .add_module(SonarFilter)?;

        #[cfg(feature = "alsa")]
        let app = app.add_module(audio_input::AudioInputFilter)?;
        Ok(app)
    }
}
