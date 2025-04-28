use std::{ops::Sub, time::Instant};

use bevy::prelude::*;
use nidhogg::{
    NaoState,
    types::{ArmJoints, FillExt, HeadJoints, LegJoints},
};
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{
    nao::{NaoManager, Priority},
    prelude::ConfigExt,
    sensor::imu::IMUValues,
};

const MOTION_MANAGER_PRIORITY: Priority = Priority::High;

pub struct MotionManagerPlugin;

impl Plugin for MotionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<GetUpBackMotionConfig>()
            .init_resource::<MotionManager>()
            .add_systems(Update, (load_next_motion, run_motion));
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
    mut nao_manager: ResMut<NaoManager>,
    nao_state: ResMut<NaoState>,
    mut angles_reached_at: Local<Option<Instant>>,
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

    if motion_config.is_complete(&imu_values, angles_reached_at.as_ref()) {
        motion_manager.next_motion();
        *angles_reached_at = None;
        return;
    }

    if motion_config
        .abort_conditions
        .iter()
        .all(|condition| condition.is_satisfied(imu_values.as_ref()))
    {
        motion_manager.abort_motion();
        return;
    }

    if !motion_config
        .angles
        .is_close(&nao_state, motion_config.angle_threshold)
    {
        if let Some(angles) = &motion_config.angles.head {
            if let Some(interpolation_weight) = motion_config.interpolation_weight {
                nao_manager.set_head_interpolate(
                    angles.clone(),
                    HeadJoints::fill(motion_config.stiffness),
                    interpolation_weight,
                    MOTION_MANAGER_PRIORITY,
                );
            } else {
                nao_manager.set_head(
                    angles.clone(),
                    HeadJoints::fill(motion_config.stiffness),
                    MOTION_MANAGER_PRIORITY,
                );
            }
        }
        if let Some(angles) = &motion_config.angles.arms {
            if let Some(interpolation_weight) = motion_config.interpolation_weight {
                nao_manager.set_arms_interpolate(
                    angles.clone(),
                    ArmJoints::fill(motion_config.stiffness),
                    interpolation_weight,
                    MOTION_MANAGER_PRIORITY,
                );
            } else {
                nao_manager.set_arms(
                    angles.clone(),
                    ArmJoints::fill(motion_config.stiffness),
                    MOTION_MANAGER_PRIORITY,
                );
            }
        }
        if let Some(angles) = &motion_config.angles.legs {
            if let Some(interpolation_weight) = motion_config.interpolation_weight {
                nao_manager.set_legs_interpolate(
                    angles.clone(),
                    LegJoints::fill(motion_config.stiffness),
                    interpolation_weight,
                    MOTION_MANAGER_PRIORITY,
                );
            } else {
                nao_manager.set_legs(
                    angles.clone(),
                    LegJoints::fill(motion_config.stiffness),
                    MOTION_MANAGER_PRIORITY,
                );
            }
        }

        return;
    }

    // If angles are correct, set `completed_motion_at` if unset.
    *angles_reached_at = Some(Instant::now());

    if motion_config.is_complete(&imu_values, angles_reached_at.as_ref()) {
        motion_manager.next_motion();
        *angles_reached_at = None;
        return;
    }
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
    interpolation_weight: Option<f32>,
    complete_conditions: Vec<Condition>,
    start_conditions: Vec<Condition>,
    min_delay: f32,
    max_delay: f32,
    angles: Joints,
    angle_threshold: f32,
    stiffness: f32,
}

impl MotionConfig {
    fn is_complete(&self, imu_values: &IMUValues, angles_reached_at: Option<&Instant>) -> bool {
        let Some(angles_reached_at) = angles_reached_at else {
            return false;
        };

        if angles_reached_at.elapsed().as_secs_f32() < self.min_delay {
            return false;
        }

        if angles_reached_at.elapsed().as_secs_f32() > self.max_delay {
            return true;
        }

        self.complete_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values))
    }
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct Joints {
    head: Option<HeadJoints<f32>>,
    arms: Option<ArmJoints<f32>>,
    legs: Option<LegJoints<f32>>,
}

impl Joints {
    fn is_close(&self, nao_state: &NaoState, threshold: f32) -> bool {
        self.head_is_close(nao_state, threshold)
            && self.arms_are_close(nao_state, threshold)
            && self.legs_are_close(nao_state, threshold)
    }

    fn head_is_close(&self, nao_state: &NaoState, threshold: f32) -> bool {
        let Some(requested_head_position) = &self.head else {
            return true;
        };

        let current_head_position = nao_state.position.head_joints();

        requested_head_position
            .clone()
            .zip(current_head_position.clone())
            .iter()
            .all(|(requested_position, current_position)| {
                requested_position
                    .sub(current_position)
                    .abs()
                    .le(&threshold)
            })
    }

    fn arms_are_close(&self, nao_state: &NaoState, threshold: f32) -> bool {
        let Some(requested_arms_position) = &self.arms else {
            return true;
        };

        let current_arms_position = nao_state.position.arm_joints();

        let left_arm_is_close = requested_arms_position
            .left_arm
            .clone()
            .zip(current_arms_position.left_arm.clone())
            .iter()
            .all(|(requested_position, current_position)| {
                requested_position
                    .sub(current_position)
                    .abs()
                    .le(&threshold)
            });

        let right_arm_is_close = requested_arms_position
            .right_arm
            .clone()
            .zip(current_arms_position.right_arm.clone())
            .iter()
            .all(|(requested_position, current_position)| {
                requested_position
                    .sub(current_position)
                    .abs()
                    .le(&threshold)
            });

        left_arm_is_close && right_arm_is_close
    }

    fn legs_are_close(&self, nao_state: &NaoState, threshold: f32) -> bool {
        let Some(requested_legs_position) = &self.legs else {
            return true;
        };

        let current_legs_position = nao_state.position.leg_joints();

        let left_leg_is_close = requested_legs_position
            .left_leg
            .clone()
            .zip(current_legs_position.left_leg.clone())
            .iter()
            .all(|(requested_position, current_position)| {
                requested_position
                    .sub(current_position)
                    .abs()
                    .le(&threshold)
            });

        let right_leg_is_close = requested_legs_position
            .right_leg
            .clone()
            .zip(current_legs_position.right_leg.clone())
            .iter()
            .all(|(requested_position, current_position)| {
                requested_position
                    .sub(current_position)
                    .abs()
                    .le(&threshold)
            });

        left_leg_is_close && right_leg_is_close
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
