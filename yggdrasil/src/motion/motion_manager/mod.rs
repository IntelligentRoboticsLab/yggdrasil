use std::{ops::Sub, time::Instant};

use bevy::prelude::*;
use nidhogg::NaoState;
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
    let Some(next_motion) = &motion_manager.next_motion.take() else {
        return;
    };

    motion_manager.motion_configs = match next_motion {
        Motion::GetUpBack => getup_back_motion_config.motions.clone(),
    };

    motion_manager.current_motion_config_id = 0;
}

fn run_motion(
    mut motion_manager: ResMut<MotionManager>,
    imu_values: Res<IMUValues>,
    completed_motion_at: Local<Option<Instant>>,
) {
    let motion_config = loop {
        let Some(motion_config) = motion_manager.current_motion() else {
            break None;
        };

        if motion_config
            .start_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values.as_ref()))
        {
            motion_manager.next_motion();
            continue;
        }

        break Some(motion_config);
    };
    let Some(motion_config) = motion_config else {
        return;
    };

    if motion_config
        .abort_conditions
        .iter()
        .all(|condition| condition.is_satisfied(imu_values.as_ref()))
    {
        motion_manager.abort_motion();
        return;
    }

    // Check joing angles for complete motion.
    //  else set joints and return

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
    motion_configs: Vec<MotionConfig>,
    current_motion_config_id: usize,

    next_motion: Option<Motion>,
}

impl MotionManager {
    pub fn set_motion_if_unset(&mut self, motion: Motion) {
        self.next_motion.get_or_insert(motion);
    }

    pub fn overwrite_motion(&mut self, motion: Motion) {
        self.next_motion = Some(motion);
    }

    pub fn abort_motion(&mut self) {
        self.motion_configs.clear();
        self.current_motion_config_id = 0;
    }

    fn current_motion<'a>(&'a self) -> Option<&'a MotionConfig> {
        self.motion_configs.get(self.current_motion_config_id)
    }

    fn next_motion(&mut self) {
        self.current_motion_config_id += 1;
        if self.current_motion_config_id == self.motion_configs.len() {
            self.current_motion_config_id = 0;
            self.motion_configs.clear();
        }
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
    },
    GiroY {
        smaller_than: Option<f32>,
        bigger_than: Option<f32>,
    },
    GiroZ {
        smaller_than: Option<f32>,
        bigger_than: Option<f32>,
    },
}

impl Condition {
    fn is_satisfied(&self, imu_values: &IMUValues) -> bool {
        match self {
            Condition::GiroX {
                smaller_than,
                bigger_than,
            } => {
                smaller_than.is_none_or(|threshold| imu_values.gyroscope.x < threshold)
                    && bigger_than.is_none_or(|threshold| imu_values.gyroscope.x > threshold)
            }
            Condition::GiroY {
                smaller_than,
                bigger_than,
            } => {
                smaller_than.is_none_or(|threshold| imu_values.gyroscope.y < threshold)
                    && bigger_than.is_none_or(|threshold| imu_values.gyroscope.y > threshold)
            }
            Condition::GiroZ {
                smaller_than,
                bigger_than,
            } => {
                smaller_than.is_none_or(|threshold| imu_values.gyroscope.z < threshold)
                    && bigger_than.is_none_or(|threshold| imu_values.gyroscope.z > threshold)
            }
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct MotionConfig {
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
    head_yaw: Option<f32>,
    right_leg_angle: Option<f32>,
    left_leg_angle: Option<f32>,
}

impl Joints {
    fn is_close(&self, nao_state: &NaoState, threshold: f32) -> bool {
        let joint_angles = nao_state.position;

        self.head_pitch
            .is_none_or(|head_pitch| head_pitch.sub(joint_angles.head_pitch).abs().le(&threshold))
            && self
                .head_yaw
                .is_none_or(|head_yaw| head_yaw.sub(joint_angles.head_yaw).abs().le(&threshold))
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
#[serde(deny_unknown_fields)]
pub struct GetUpBackMotionConfig {
    motions: Vec<MotionConfig>,
}

impl Config for GetUpBackMotionConfig {
    const PATH: &'static str = "motions/get_up_back.toml";
}
