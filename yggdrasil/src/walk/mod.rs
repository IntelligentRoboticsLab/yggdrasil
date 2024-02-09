pub mod engine;
pub mod smoothing;
pub mod states;

use std::time::Duration;

use crate::prelude::*;
use nidhogg::types::{Vector2, Vector3};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{filter, nao, primary_state};

use self::engine::WalkingEngine;

/// Filtered gyroscope values.
#[derive(Default, Debug, Clone)]
pub struct FilteredGyroscope(Vector2<f32>);

impl FilteredGyroscope {
    pub fn update(&mut self, gyroscope: &Vector3<f32>) {
        self.0.x = 0.8 * self.0.x + 0.2 * gyroscope.x;
        self.0.y = 0.8 * self.0.y + 0.2 * gyroscope.y;
    }

    pub fn reset(&mut self) {
        self.0 = Vector2::default();
    }

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalancingConfig {
    pub arm_swing_multiplier: f32,
    pub filtered_gyro_y_multiplier: f32,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct WalkingEngineConfig {
    #[serde_as(as = "DurationMilliSeconds")]
    pub base_step_period: Duration,
    pub com_multiplier: f32,
    pub cop_pressure_threshold: f32,
    pub base_foot_lift: f32,
    pub hip_height: f32,
    pub sitting_hip_height: f32,
    pub balancing: BalancingConfig,
}

impl Config for WalkingEngineConfig {
    const PATH: &'static str = "walking_engine.toml";
}

/// A module providing the walking engine for the robot.
///
/// This module provides the following resources to the application:
/// - [`WalkingEngine`]
/// - [`FilteredGyroscope`]
pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_config::<WalkingEngineConfig>()?
            .init_resource::<FilteredGyroscope>()?
            .add_startup_system(init_walking_engine)?
            .add_system(
                filter_gyro_values
                    .after(nao::write_hardware_info)
                    .after(filter::imu::imu_filter),
            )
            .add_system(
                engine::walking_engine
                    .before(primary_state::update_primary_state)
                    .after(nao::update_cycle_time)
                    .after(filter_gyro_values)
                    .after(filter::fsr::force_sensitive_resistor_filter),
            )
            .add_system(
                engine::toggle_walking_engine
                    .before(primary_state::update_primary_state)
                    .after(filter::button::button_filter)
                    .before(engine::walking_engine),
            ))
    }
}

#[startup_system]
fn init_walking_engine(storage: &mut Storage, config: &WalkingEngineConfig) -> Result<()> {
    storage.add_resource(Resource::new(WalkingEngine::new(config)))
}

#[system]
fn filter_gyro_values(
    imu_values: &filter::imu::IMUValues,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    filtered_gyro.update(&imu_values.gyroscope);

    Ok(())
}
