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
use serde_with::{DurationSeconds, serde_as};

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

    motion_manager.key_frames = match next_motion {
        Motion::GetUpBack => getup_back_motion_config.key_frames.clone(),
    };

    motion_manager.current_key_frame_id = 0;
    let key_frame_duration = motion_manager
        .current_key_frame()
        .map(|key_frame| key_frame.duration)
        .unwrap_or(Duration::ZERO);
    motion_manager.joint_interpolator = JointInterpolator::new(key_frame_duration);
}

fn run_motion(
    mut motion_manager: ResMut<MotionManager>,
    imu_values: Res<IMUValues>,
    mut nao_manager: ResMut<NaoManager>,
    nao_state: ResMut<NaoState>,
    mut angles_reached_at: Local<Option<Instant>>,
) {
    eprintln!("keyframe id: {}", motion_manager.current_key_frame_id);
    let key_frame = loop {
        eprintln!("loop");
        let Some(key_frame) = motion_manager.current_key_frame() else {
            eprintln!("break none");
            break None;
        };

        if !key_frame
            .start_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values.as_ref()))
        {
            motion_manager.next_key_frame(nao_state.position.clone());
            *angles_reached_at = None;
            eprintln!("continue");
            continue;
        }

        eprintln!("break some");
        break Some(key_frame);
    };
    let Some(key_frame) = key_frame else {
        eprintln!("return");
        return;
    };

    // TODO: Is this check still necessary?
    if key_frame.is_complete(&imu_values, angles_reached_at.as_ref()) {
        eprintln!("KEY FRAME COMPLETE, GOING NEXT KEY FRAME 1");
        motion_manager.next_key_frame(nao_state.position.clone());
        *angles_reached_at = None;
        return;
    }
    eprintln!("key frame not complete");

    if !key_frame.abort_conditions.is_empty()
        && key_frame
            .abort_conditions
            .iter()
            .all(|condition| condition.is_satisfied(imu_values.as_ref()))
    {
        eprintln!("ABORTING MOTION");
        motion_manager.abort_motion();
        return;
    }
    eprintln!("not aborting");

    let mut close = true;
    if !key_frame
        .angles
        .head_is_close(&nao_state, key_frame.angle_threshold)
    {
        eprintln!("head not close");
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
        close = false;
    }
    if !key_frame
        .angles
        .arms_are_close(&nao_state, key_frame.angle_threshold)
    {
        eprintln!("arms not close");
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
        close = false;
    }
    if !key_frame
        .angles
        .legs_are_close(&nao_state, key_frame.angle_threshold)
    {
        eprintln!("legs not close");
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
        close = false;
    }

    if !close {
        eprintln!("!close");
        return;
    }

    // If angles are correct, set `completed_motion_at` if unset.
    *angles_reached_at = Some(Instant::now());

    if key_frame.is_complete(&imu_values, angles_reached_at.as_ref()) {
        eprintln!("KEY FRAME COMPLETE, GOING NEXT KEY FRAME 2");
        motion_manager.next_key_frame(nao_state.position.clone());
        *angles_reached_at = None;
        return;
    }

    eprintln!("key frame not complete");
}

#[derive(Resource)]
pub struct MotionManager {
    key_frames: Vec<KeyFrame>,
    current_key_frame_id: usize,

    next_motion: Option<Motion>,
    joint_interpolator: JointInterpolator,
    key_frame_start_joint_angles: JointArray<f32>,
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

        let Some(next_motion) = self.current_key_frame() else {
            self.current_key_frame_id = 0;
            self.key_frames.clear();
            std::process::exit(0);
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
    #[serde_as(as = "DurationSeconds<f64>")]
    duration: Duration,
    complete_conditions: Vec<Condition>,
    start_conditions: Vec<Condition>,
    min_delay: f32,
    max_delay: f32,
    angles: Joints,
    angle_threshold: f32,
    stiffness: f32,
}

impl KeyFrame {
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
                // eprintln!(
                //     "head joints difference (requested - current): ({} - {}).abs() =  {}",
                //     requested_position,
                //     current_position,
                //     requested_position.sub(current_position).abs()
                // );
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
                // eprintln!(
                //     "left arm joints difference (requested - current): ({} - {}).abs() =  {}",
                //     requested_position,
                //     current_position,
                //     requested_position.sub(current_position).abs()
                // );
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
                // eprintln!(
                //     "right arm joints difference (requested - current): ({} - {}).abs() =  {}",
                //     requested_position,
                //     current_position,
                //     requested_position.sub(current_position).abs()
                // );
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
                // eprintln!(
                //     "left leg joints difference (requested - current): ({} - {}).abs() =  {}",
                //     requested_position,
                //     current_position,
                //     requested_position.sub(current_position).abs()
                // );
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
                // eprintln!(
                //     "right leg joints difference (requested - current): ({} - {}).abs() =  {}",
                //     requested_position,
                //     current_position,
                //     requested_position.sub(current_position).abs()
                // );
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
    key_frames: Vec<KeyFrame>,
}

impl Config for GetUpBackMotionConfig {
    const PATH: &'static str = "motions/get_up_back.toml";
}
