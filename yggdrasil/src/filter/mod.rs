use crate::prelude::*;

use self::{
    button::ButtonFilter, falling::FallingFilter, fsr::FSRFilter, imu::IMUFilter,
    sonar::SonarFilter,
};

#[cfg(feature = "alsa")]
pub mod audio_input;
pub mod button;
pub mod falling;
pub mod fsr;
pub mod imu;
pub mod sonar;

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
