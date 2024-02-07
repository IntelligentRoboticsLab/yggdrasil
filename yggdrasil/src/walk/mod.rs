pub mod engine;
pub mod smoothing;
mod states;

use std::ops::Add;

use miette::Result;

use nidhogg::types::{Vector2, Vector3};
use tyr::prelude::*;

use crate::{filter, nao, primary_state};

use self::engine::WalkingEngine;

#[derive(Default, Debug, Clone)]
pub struct Odometry {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

impl Add for Odometry {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            forward: self.forward + rhs.forward,
            left: self.left + rhs.left,
            turn: self.turn + rhs.turn,
        }
    }
}

/// Filtered gyroscope values
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

/// A module providing the walking engine for the robot.
///
/// This module provides the following resources to the application:
/// - [`WalkingEngine`]
/// - [`FilteredGyroscope`]
pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<WalkingEngine>()?
            .init_resource::<FilteredGyroscope>()?
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

#[system]
fn filter_gyro_values(
    imu_values: &filter::imu::IMUValues,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    filtered_gyro.update(&imu_values.gyroscope);

    Ok(())
}
