use crate::prelude::*;

use self::{
    button::ButtonFilter, falling::FallingFilter, fsr::FSRFilter, imu::IMUFilter,
    sonar::SonarFilter,
};

pub mod button;
pub mod falling;
pub mod fsr;
pub mod imu;
pub mod sonar;

pub struct FilterModule;

impl Module for FilterModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(ButtonFilter)?
            .add_module(FSRFilter)?
            .add_module(IMUFilter)?
            .add_module(SonarFilter)?
            .add_module(FallingFilter)
    }
}
