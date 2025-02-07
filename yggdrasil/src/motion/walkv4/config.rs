use std::time::Duration;

use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use super::{foot_support::FootSupportConfig, step::Step};

#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
pub struct BalancingConfig {
    pub arm_swing_multiplier: f32,
    pub filtered_gyro_y_multiplier: f32,
}

/// Configuration for the walking engine.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WalkingEngineConfig {
    #[serde_as(as = "DurationMilliSeconds")]
    pub base_step_period: Duration,
    pub leg_stiffness: f32,
    pub arm_stiffness: f32,
    pub cop_pressure_threshold: f32,
    pub base_foot_lift: f32,
    pub foot_lift_modifier: Step,
    pub max_step_size: Step,
    pub hip_height: f32,
    pub max_sitting_hip_height: f32,
    pub balancing: BalancingConfig,
    pub foot_support: FootSupportConfig,
}

impl Config for WalkingEngineConfig {
    const PATH: &'static str = "walking_engine.toml";
}
