use std::time::Instant;

use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{prelude::ConfigExt, sensor::imu::IMUValues};

pub struct MotionManagerPlugin;

impl Plugin for MotionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<GetUpBackMotionConfig>()
            .init_resource::<MotionManager>()
            .add_systems(Update, load_next_motion);
    }
}

fn load_next_motion(
    mut motion_manager: ResMut<MotionManager>,
    getup_back_motion_config: Res<GetUpBackMotionConfig>,
) {
    let Some(next_motion) = &motion_manager.next_motion else {
        return;
    };

    match next_motion {
        Motion::GetUpBack => {
            motion_manager.current_motions = Some((0, getup_back_motion_config.motions.clone()));
            motion_manager.next_motion = None;
        }
    }
}

fn run_motion(
    mut motion_manager: ResMut<MotionManager>,
    imu_values: Res<IMUValues>,
    completed_motion_at: Local<Option<Instant>>,
) {
    let Some((current_motion_id, motions)) = motion_manager.current_motions.as_ref() else {
        return;
    };

    let current_motion = &motions[*current_motion_id];

    // Check joing angles for complete motion.
    //  else return

    // If angles are correct, set `completed_motion_at` if unset.

    // Check `end_conditions`,
    //  if (end_conditions.all() == true && completed_motion_at.elapsed > min_delay) ||
    //  completed_motion_at.elapsed > max_delay {
    //
    //  } else {
    //      continue;
    //  }

    // Set next motion.
}

#[derive(Default, Resource)]
pub struct MotionManager {
    current_motions: Option<(usize, Vec<Motions>)>,
    next_motion: Option<Motion>,
}

impl MotionManager {
    pub fn set_motion_if_unset(&mut self, motion: Motion) {
        self.next_motion.get_or_insert(motion);
    }

    pub fn overwrite_motion(&mut self, motion: Motion) {
        self.next_motion = Some(motion);
    }

    fn current_motion<'a>(&'a self) -> Option<&'a Motions> {
        self.current_motions
            .iter()
            .flat_map(|(current_motion_id, motions)| motions.get(*current_motion_id))
            .next()
    }

    fn next_motion(&mut self) {
        let Some((current_motion_id, motions)) = self.current_motions.as_mut() else {
            return;
        };

        *current_motion_id += 1;
        if *current_motion_id == motions.len() {
            *current_motion_id = 0;
            motions.clear();
        }
        return;
    }
}

pub enum Motion {
    GetUpBack,
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
