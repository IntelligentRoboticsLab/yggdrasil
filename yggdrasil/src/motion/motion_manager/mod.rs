mod joint_interpolator;

use std::{
    ops::Sub,
    time::{Duration, Instant},
};

use bevy::prelude::*;
use joint_interpolator::JointInterpolator;
use nidhogg::{
    NaoState,
    types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints},
};
use odal::Config;
use serde::{Deserialize, Serialize};
use serde_with::{DurationSecondsWithFrac, serde_as};

use crate::{
    nao::{NaoManager, Priority},
    prelude::ConfigExt,
    sensor::imu::IMUValues,
};

const MOTION_MANAGER_PRIORITY: Priority = Priority::Custom(91);

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
    nao_state: ResMut<NaoState>,
    // TODO: Remove after testing.
    mut already_run: Local<bool>,
) {
    if *already_run {
        return;
    }

    let Some(next_motion) = &motion_manager.next_motion.take() else {
        return;
    };

    motion_manager.key_frames = match next_motion {
        Motion::GetUpBack => getup_back_motion_config.key_frames.clone(),
    };

    motion_manager.current_key_frame_id = 0;
    let key_frame_duration = motion_manager
        .current_key_frame()
        .map(|key_frame| key_frame.duration)
        .unwrap();
    // assert_ne!(key_frame_duration, Duration::from_secs(0));
    // assert!(false);
    motion_manager.joint_interpolator = JointInterpolator::new(key_frame_duration);
    motion_manager.key_frame_start_joint_angles = nao_state.position.clone();
    motion_manager.key_frame_start = Instant::now();

    *already_run = true;
}

fn run_motion(
    mut motion_manager: ResMut<MotionManager>,
    imu_values: Res<IMUValues>,
    mut nao_manager: ResMut<NaoManager>,
    nao_state: ResMut<NaoState>,
    mut angles_reached_at: Local<Option<Instant>>,
) {
    let key_frame = loop {
        let Some(key_frame) = motion_manager.current_key_frame() else {
            break None;
        };

        if !key_frame
            .start_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values.as_ref()))
        {
            motion_manager.next_key_frame(nao_state.position.clone());
            *angles_reached_at = None;
            continue;
        }

        break Some(key_frame);
    };
    let Some(key_frame) = key_frame else {
        return;
    };

    // // TODO: Is this check still necessary?
    // if key_frame.is_complete(&imu_values, angles_reached_at.as_ref()) {
    //     motion_manager.next_key_frame(nao_state.position.clone());
    //     *angles_reached_at = None;
    //     return;
    // }

    if !key_frame.abort_conditions.is_empty()
        && key_frame
            .abort_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values.as_ref()))
    {
        motion_manager.abort_motion();
        return;
    }

    if let Some(angles) = &key_frame.angles.head {
        let target_angles = motion_manager.joint_interpolator.interpolated_positions(
            motion_manager.key_frame_start_joint_angles.head_joints(),
            angles.clone(),
        );
        nao_manager.set_head(
            target_angles,
            HeadJoints::fill(key_frame.stiffness),
            MOTION_MANAGER_PRIORITY,
        );
    }
    if let Some(angles) = &key_frame.angles.arms {
        let target_angles = motion_manager.joint_interpolator.interpolated_positions(
            motion_manager.key_frame_start_joint_angles.arm_joints(),
            angles.clone(),
        );
        nao_manager.set_arms(
            target_angles,
            ArmJoints::fill(key_frame.stiffness),
            MOTION_MANAGER_PRIORITY,
        );
    }
    if let Some(angles) = &key_frame.angles.legs {
        let target_angles = motion_manager.joint_interpolator.interpolated_positions(
            motion_manager.key_frame_start_joint_angles.leg_joints(),
            angles.clone(),
        );
        nao_manager.set_legs(
            target_angles,
            LegJoints::fill(key_frame.stiffness),
            MOTION_MANAGER_PRIORITY,
        );
    }

    // TODO: Also check for complete conditions.
    if motion_manager.key_frame_start.elapsed()
        <= key_frame.duration + Duration::from_secs_f32(key_frame.min_delay)
    {
        return;
    }

    // If angles are correct, set `completed_motion_at` if unset.
    angles_reached_at.get_or_insert(Instant::now());

    if key_frame.is_complete(&imu_values, angles_reached_at.unwrap()) {
        motion_manager.next_key_frame(nao_state.position.clone());
        *angles_reached_at = None;
        return;
    }
}

#[derive(Resource)]
pub struct MotionManager {
    key_frames: Vec<KeyFrame>,
    current_key_frame_id: usize,

    next_motion: Option<Motion>,
    joint_interpolator: JointInterpolator,
    key_frame_start_joint_angles: JointArray<f32>,
    key_frame_start: Instant,
}

impl MotionManager {
    pub fn set_motion_if_unset(&mut self, motion: Motion) {
        self.next_motion.get_or_insert(motion);
    }

    pub fn in_motion(&self) -> bool {
        !self.key_frames.is_empty()
    }

    pub fn set_motion_if_not_running(&mut self, motion: Motion) {
        if self.in_motion() {
            return;
        }

        self.set_motion_if_unset(motion);
    }

    pub fn overwrite_motion(&mut self, motion: Motion) {
        self.next_motion = Some(motion);
    }

    pub fn abort_motion(&mut self) {
        self.key_frames.clear();
        self.current_key_frame_id = 0;
    }

    fn current_key_frame<'a>(&'a self) -> Option<&'a KeyFrame> {
        self.key_frames.get(self.current_key_frame_id)
    }

    fn next_key_frame(&mut self, current_joint_angles: JointArray<f32>) {
        self.current_key_frame_id += 1;
        self.key_frame_start = Instant::now();

        let Some(next_motion) = self.current_key_frame() else {
            self.current_key_frame_id = 0;
            self.key_frames.clear();
            return;
        };

        self.joint_interpolator = JointInterpolator::new(next_motion.duration);
        self.key_frame_start_joint_angles = current_joint_angles;
    }
}

impl Default for MotionManager {
    fn default() -> Self {
        Self {
            key_frames: Vec::new(),
            current_key_frame_id: 0,

            next_motion: None,
            joint_interpolator: JointInterpolator::new(Duration::ZERO),
            key_frame_start_joint_angles: JointArray::default(),

            key_frame_start: Instant::now(),
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
struct KeyFrame {
    abort_conditions: Vec<Condition>,
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    duration: Duration,
    complete_conditions: Vec<Condition>,
    start_conditions: Vec<Condition>,
    min_delay: f32,
    max_delay: f32,
    angles: Joints,
    stiffness: f32,
}

impl KeyFrame {
    fn is_complete(&self, imu_values: &IMUValues, angles_reached_at: Instant) -> bool {
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

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
#[serde(deny_unknown_fields)]
pub struct GetUpBackMotionConfig {
    key_frames: Vec<KeyFrame>,
}

impl Config for GetUpBackMotionConfig {
    const PATH: &'static str = "motions/get_up_back.toml";
}
