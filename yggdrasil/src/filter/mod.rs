use color_eyre::Result;
use tyr::prelude::*;

use self::{button::ButtonFilter, fsr::FSRFilter, imu::IMUFilter, sonar::SonarFilter};

pub mod button;
pub mod fsr;
pub mod imu;
pub mod sonar;

pub struct FilterModule;

impl Module for FilterModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_module(ButtonFilter)?
            .add_module(FSRFilter)?
            .add_module(IMUFilter)?
            .add_module(SonarFilter)?)
    }
}
