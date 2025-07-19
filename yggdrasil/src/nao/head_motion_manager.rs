use std::time::Instant;

use bevy::prelude::*;
use nalgebra::Point3;
use nidhogg::types::{FillExt, HeadJoints};

use crate::behavior::BehaviorConfig;
use crate::localization::RobotPose;
use crate::nao::NaoManager;
use crate::nao::Priority;

pub(super) struct HeadMotionManagerPlugin;

impl Plugin for HeadMotionManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeadMotionManager>()
            .init_state::<HeadMotionState>()
            .add_systems(PreUpdate, update_head_motion_state)
            .add_systems(
                Update,
                (
                    fixed_head.run_if(in_state(HeadMotionState::FixedHead)),
                    look_at.run_if(in_state(HeadMotionState::LookAt)),
                    look_around.run_if(in_state(HeadMotionState::LookAround)),
                ),
            );
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub(crate) enum HeadMotionState {
    #[default]
    Neutral,
    FixedHead,
    LookAround,
    LookAt,
}

#[derive(Default)]
pub(crate) enum HeadMotionRequest {
    #[default]
    Neutral,
    FixedHead(FixedHead),
    LookAround,
    LookAt(LookAt),
}

#[derive(Resource, Default)]
pub(crate) struct HeadMotionManager {
    requested_head_motion_state: HeadMotionState,
    requested_head_motion_settings: HeadMotionRequest,
    look_around_starting_time: Option<Instant>,
}

impl HeadMotionManager {
    pub(crate) fn request_fixed_head(&mut self, fixed_head: FixedHead) {
        self.requested_head_motion_state = HeadMotionState::FixedHead;
        self.requested_head_motion_settings = HeadMotionRequest::FixedHead(fixed_head);
    }

    pub(crate) fn request_look_at(&mut self, look_at: LookAt) {
        self.requested_head_motion_state = HeadMotionState::LookAt;
        self.requested_head_motion_settings = HeadMotionRequest::LookAt(look_at);
    }

    pub(crate) fn request_look_around(&mut self) {
        if self.requested_head_motion_state != HeadMotionState::LookAround {
            self.look_around_starting_time = Some(Instant::now());
        }

        self.requested_head_motion_settings = HeadMotionRequest::LookAround;
        self.requested_head_motion_state = HeadMotionState::LookAround;
    }
}

fn update_head_motion_state(
    head_motion_manager: Res<HeadMotionManager>,
    mut head_motion_state: ResMut<NextState<HeadMotionState>>,
) {
    head_motion_state.set(head_motion_manager.requested_head_motion_state);
}

#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct LookAt {
    pub(crate) pose: RobotPose,
    pub(crate) point: Point3<f32>,
}

/// Head motion where the head will position it self to look at the given point
fn look_at(
    mut nao_manager: ResMut<NaoManager>,
    head_motion_manager: Res<HeadMotionManager>,
    mut look_at: Local<LookAt>,
    behavior_config: Res<BehaviorConfig>,
) {
    // Update the look_at data if a new request of the look_at motion type was requested
    if let HeadMotionRequest::LookAt(look_at_request) =
        head_motion_manager.requested_head_motion_settings
    {
        *look_at = look_at_request;
    }

    let observe_config = &behavior_config.observe;

    let joint_positions = look_at.pose.get_look_at_absolute(&look_at.point);
    let joint_stiffness = HeadJoints::fill(observe_config.look_at_head_stiffness);

    nao_manager.set_head(joint_positions, joint_stiffness, Priority::default());
}

/// `FixedHead` resource which contains the data, used in the fixed head motion system
#[derive(Resource, Default, Clone, Copy)]
pub(crate) struct FixedHead {
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
    pub(crate) stiffness: f32,
    pub(crate) priority: Priority,
}

fn fixed_head(
    mut nao_manager: ResMut<NaoManager>,
    head_motion_manager: Res<HeadMotionManager>,
    mut fixed_head: Local<FixedHead>,
) {
    // Update the head motion data if a new request of the fixed head motion type was requested
    if let HeadMotionRequest::FixedHead(fixed_head_request) =
        head_motion_manager.requested_head_motion_settings
    {
        *fixed_head = fixed_head_request;
    }

    let joint_positions = HeadJoints {
        yaw: fixed_head.yaw,
        pitch: fixed_head.pitch,
    };
    let joint_stiffness = HeadJoints::fill(fixed_head.stiffness);

    nao_manager.set_head(joint_positions, joint_stiffness, fixed_head.priority);
}

fn look_around(
    mut nao_manager: ResMut<NaoManager>,
    head_motion_manager: Res<HeadMotionManager>,
    behavior_config: Res<BehaviorConfig>,
) {
    let observe_config = &behavior_config.observe;

    let Some(starting_time) = head_motion_manager.look_around_starting_time else {
        return;
    };

    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress =
        starting_time.elapsed().as_secs_f32() * observe_config.head_rotation_speed;

    let yaw = (movement_progress).sin() * observe_config.head_yaw_max;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * observe_config.head_pitch_max;

    let joint_positions = HeadJoints { yaw, pitch };
    let joint_stiffness = HeadJoints::fill(observe_config.look_around_head_stiffness);

    nao_manager.set_head(joint_positions, joint_stiffness, Priority::default());
}
