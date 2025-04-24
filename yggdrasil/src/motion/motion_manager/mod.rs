use std::{ops::Sub, time::Instant};

use bevy::prelude::*;
use nidhogg::types::{ArmJoints, HeadJoints, LegJoints};
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

    if !motion_config
        .angles
        .is_close(&nao_manager, motion_config.angle_threshold)
    {
        if let (Some(angles), Some(stiffness)) =
            (&motion_config.angles.head, &motion_config.stiffness.head)
        {
            nao_manager.set_head(angles.clone(), stiffness.clone(), MOTION_MANAGER_PRIORITY);
        }
        if let (Some(angles), Some(stiffness)) =
            (&motion_config.angles.arms, &motion_config.stiffness.arms)
        {
            nao_manager.set_arms(angles.clone(), stiffness.clone(), MOTION_MANAGER_PRIORITY);
        }
        if let (Some(angles), Some(stiffness)) =
            (&motion_config.angles.legs, &motion_config.stiffness.legs)
        {
            nao_manager.set_legs(angles.clone(), stiffness.clone(), MOTION_MANAGER_PRIORITY);
        }

        return;
    }

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
    angle_threshold: f32,
    stiffness: Joints,
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
    fn is_close(&self, nao_manager: &NaoManager, threshold: f32) -> bool {
        self.head_is_close(nao_manager, threshold)
        // TODO: Arms
        // TODO: LEGS
    }

    fn head_is_close(&self, nao_manager: &NaoManager, threshold: f32) -> bool {
        let head_angles = nao_manager.head_position();

        self.head
            .as_ref()
            .is_none_or(|head| head.pitch.sub(head_angles.pitch).abs().le(&threshold))
            && self
                .head
                .as_ref()
                .is_none_or(|head| head.yaw.sub(head_angles.yaw).abs().le(&threshold))
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
