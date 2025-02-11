use std::time::Duration;

use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use super::{foot_support::FootSupportConfig, hips::HipHeightConfig, step::Step};

#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
pub struct BalancingConfig {
    /// The amount to swing the arms based on the forward movement.
    pub arm_swing_multiplier: f32,
    /// The alpha parameter used for the low pass filter over the gyroscope values.
    ///
    /// Higher values mean that the filtered gyroscope value responds to changes quicker,
    /// and lower values mean that it responds slower.
    pub gyro_lpf_alpha: f32,
    /// The weight of the balance adjustment based on the y gyroscope value.
    ///
    /// Increasing this value will use a larger portion of the filtered gyro value
    /// to adjust the pitch of the robot's ankles, in order to balance the pendulum motion.
    pub filtered_gyro_y_multiplier: f32,
}

/// Configuration for the walking engine.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct WalkingEngineConfig {
    /// The base amount of time (in milliseconds) for one step.
    #[serde_as(as = "DurationMilliSeconds")]
    pub base_step_duration: Duration,

    /// Duration modifiers (in seconds) for a single step.
    /// Modifies `base_step_duration` by adding (modifier * movement factor) seconds per step.
    pub step_duration_modifier: Step,

    /// The offset of the torso w.r.t. the hips in meters.
    ///
    /// Higher values will result in the robot leaning forward while walking.
    /// Negative values will make the robot lean backwards while walking.
    pub torso_offset: f32,

    /// The stiffness value used for the leg joints, higher means the robot's joints will
    /// wear out faster, but the robot will be more stable.
    pub walking_leg_stiffness: f32,

    /// The stiffness value used for the leg joints while walking, higher means the robot's joints will
    /// wear out faster, but the robot will be more stable.
    ///
    /// Negative values will turn the motors off completely, sacrificing all stability.
    pub sitting_leg_stiffness: f32,

    /// The amount of time (in milliseconds) of no change in hip height
    /// for the robot to be considered stable when sitting down.
    #[serde_as(as = "DurationMilliSeconds")]
    pub stable_sitting_timeout: Duration,

    /// The stiffness value used for the arm joints, higher means the robot's joints will
    /// wear out faster, but the robot will be more stable.
    pub arm_stiffness: f32,

    /// The base amount to lift the feet in swing phase, in metres.
    /// The foot lift is increased slightly, based on the forward and left in the command.
    pub base_foot_lift: f32,

    /// The multiplier for each component of a step command to adjust the foot lift.
    /// These values are multiplied by their respective component of the step command
    /// and added to the base foot lift.
    pub foot_lift_modifier: Step,

    /// The step size is clipped to this value; in both directions
    /// (e.g. range for forward is -max_step_size to max_step_size).
    pub max_step_size: Step,

    /// Balancing parameters
    pub balancing: BalancingConfig,

    /// Foot support parameters
    pub foot_support: FootSupportConfig,

    /// Hip height parameters
    pub hip_height: HipHeightConfig,
}

impl Config for WalkingEngineConfig {
    const PATH: &'static str = "walking_engine.toml";
}
