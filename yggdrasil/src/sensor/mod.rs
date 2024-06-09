use crate::prelude::*;

use self::{
    button::ButtonFilter, falling::FallingFilter, fsr::FSRSensor, imu::IMUSensor,
    orientation::OrientationFilter, sonar::SonarSensor,
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
pub mod orientation;
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

pub struct SensorModule;

impl Module for SensorModule {
    fn initialize(self, app: App) -> Result<App> {
        let app = app
            .add_module(ButtonFilter)?
            .add_module(FSRSensor)?
            .add_module(IMUSensor)?
            .add_module(OrientationFilter)?
            .add_module(FallingFilter)?
            .add_module(SonarSensor)?;
        Ok(app)
    }
}
