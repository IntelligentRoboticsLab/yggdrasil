use bevy::{app::PluginGroupBuilder, prelude::*};
use serde::{Deserialize, Serialize};

pub mod button;
pub mod falling;
pub mod foot_bumpers;
pub mod fsr;
pub mod imu;
pub mod low_pass_filter;
pub mod orientation;
pub mod sonar;

/// Plugin group for all sensor related plugins.
pub struct SensorPlugins;

impl PluginGroup for SensorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(button::ButtonPlugin)
            .add(foot_bumpers::FootBumperPlugin)
            .add(fsr::FSRSensorPlugin)
            .add(imu::IMUSensorPlugin)
            .add(sonar::SonarSensorPlugin)
            .add(orientation::OrientationFilterPlugin)
            .add(falling::FallingFilterPlugin)
    }
}

/// Configuration for all sensor related plugins.
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SensorConfig {
    /// Configuration for the FSR sensor.
    pub fsr: fsr::FsrConfig,

    /// Configuration for the button sensitivies.
    pub button: button::ButtonConfig,

    /// Configuration for the foot bumpers.
    pub foot_bumpers: foot_bumpers::FootBumperConfig,
}
