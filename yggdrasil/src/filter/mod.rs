use miette::Result;
use tyr::prelude::*;

use self::{button::ButtonFilter, fsr::FSRFilter, imu::IMUFilter, sonar::SonarFilter, input_audio::InputAudioFilter};

pub mod button;
pub mod fsr;
pub mod imu;
pub mod sonar;
pub mod input_audio;

pub struct FilterModule;

impl Module for FilterModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(ButtonFilter)?
            .add_module(FSRFilter)?
            .add_module(IMUFilter)?
            .add_module(SonarFilter)?
            .add_module(InputAudioFilter)
    }
}
