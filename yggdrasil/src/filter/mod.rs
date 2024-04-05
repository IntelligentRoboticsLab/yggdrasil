use crate::prelude::*;

use self::{
    button::ButtonFilter, falling::FallingFilter, fsr::FSRFilter, imu::IMUFilter,
    sonar::SonarFilter,
};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

pub mod button;
pub mod falling;
pub mod fsr;
pub mod imu;
/// A simple low pass smoothing filter.
pub mod low_pass_filter;
pub mod sonar;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FilterConfig {
    pub fsr: FsrConfig,
    pub button: ButtonConfig,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ButtonConfig {
    pub activation_threshold: f32,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub held_duration_threshold: Duration,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FsrConfig {
    pub ground_contact_threshold: f32,
}

pub struct FilterModule;

impl Module for FilterModule {
    fn initialize(self, app: App) -> Result<App> {
        let app = app
            .add_module(ButtonFilter)?
            .add_module(FSRFilter)?
            .add_module(IMUFilter)?
            .add_module(FallingFilter)?
            .add_module(SonarFilter)?;
        Ok(app)
    }
}
