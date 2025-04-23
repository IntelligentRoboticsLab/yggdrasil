use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{prelude::ConfigExt, sensor::imu::IMUValues};

pub struct MotionManagerPlugin;

impl Plugin for MotionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<GetUpBackMotionConfig>();
    }
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// #[serde(deny_unknown_fields)]
// // #[serde(tag = "field")]
// #[serde(untagged)]
// enum Condition {
//     Range {
//         field: String,
//         smaller_than: f32,
//         bigger_than: f32,
//         contains: Option<bool>,
//     },
//     Smaller {
//         field: String,
//         smaller_than: f32,
//     },
//     BiggerThan {
//         field: String,
//         bigger_than: f32,
//     },
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
#[serde(tag = "field")]
enum Condition {
    GiroX {
        smaller_than: Option<f32>,
        bigger_than: Option<f32>,
        contains: Option<bool>,
    },
    GiroY {
        smaller_than: Option<f32>,
        bigger_than: Option<f32>,
        contains: Option<bool>,
    },
    GiroZ {
        smaller_than: Option<f32>,
        bigger_than: Option<f32>,
        contains: Option<bool>,
    },
}

impl Condition {
    fn is_satisfied(&self, imu_values: &IMUValues) -> bool {
        match self {
            Condition::GiroX {
                smaller_than,
                bigger_than,
                contains,
            } => {
                (smaller_than
                    .map(|threshold| imu_values.gyroscope.x < threshold)
                    .unwrap_or(true)
                    && bigger_than
                        .map(|threshold| imu_values.gyroscope.x > threshold)
                        .unwrap_or(true))
                    ^ contains.unwrap_or_default()
            }
            Condition::GiroY {
                smaller_than,
                bigger_than,
                contains,
            } => {
                (smaller_than
                    .map(|threshold| imu_values.gyroscope.y < threshold)
                    .unwrap_or(true)
                    && bigger_than
                        .map(|threshold| imu_values.gyroscope.y > threshold)
                        .unwrap_or(true))
                    ^ contains.unwrap_or_default()
            }
            Condition::GiroZ {
                smaller_than,
                bigger_than,
                contains,
            } => {
                (smaller_than
                    .map(|threshold| imu_values.gyroscope.z < threshold)
                    .unwrap_or(true)
                    && bigger_than
                        .map(|threshold| imu_values.gyroscope.z > threshold)
                        .unwrap_or(true))
                    ^ contains.unwrap_or_default()
            }
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Motions {
    abort_conditions: Vec<Condition>,
    interpolate: bool,
    end_conditions: Vec<Condition>,
    start_conditions: Vec<Condition>,
    min_delay: f32,
    max_delay: f32,
    angles: Joints,
    stiffness: Joints,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Joints {
    head_pitch: Option<f32>,
    head_jaw: Option<f32>,
    right_leg_angle: Option<f32>,
    left_leg_angle: Option<f32>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
#[serde(deny_unknown_fields)]
pub struct GetUpBackMotionConfig {
    motions: Vec<Motions>,
}

impl Config for GetUpBackMotionConfig {
    const PATH: &'static str = "motions/get_up_back.toml";
}
