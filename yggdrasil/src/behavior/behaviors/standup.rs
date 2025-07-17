use std::time::Instant;

use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::{
        keyframe::{KeyframeExecutor, MotionType},
        walking_engine::step_context::{self, StepContext},
    },
    nao::{NaoManager, Priority},
    sensor::falling::{FallState, LyingDirection},
};

/// Behavior dedicated to handling the getup sequence of the robot.
/// The behavior will be entered once the robot is confirmed to be lying down,
/// this will execute the appropriate standup motion after which the robot will return
/// to the appropriate next behavior.
#[derive(Resource, Default)]
pub struct Standup {
    completed: bool,
}

impl Standup {
    #[must_use]
    pub fn completed(&self) -> bool {
        self.completed
    }
}

impl Behavior for Standup {
    const STATE: BehaviorState = BehaviorState::Standup;
}
pub struct StandupBehaviorPlugin;

impl Plugin for StandupBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, standup.run_if(in_behavior::<Standup>));
    }
}

fn standup(
    mut standup: ResMut<Standup>,
    fall_state: Res<FallState>,
    mut keyframe_executor: ResMut<KeyframeExecutor>,
    mut completed: Local<Option<Instant>>,
    mut nao_manager: ResMut<NaoManager>,
    mut step_context: ResMut<StepContext>,
) {
    // check the direction the robot is lying and execute the appropriate motion
    match fall_state.as_ref() {
        FallState::Lying(LyingDirection::FacingDown) => {
            keyframe_executor.start_new_motion(MotionType::StandupStomach, Priority::High);
        }
        FallState::Lying(LyingDirection::FacingUp) => {
            keyframe_executor.start_new_motion(MotionType::StandupBack, Priority::High);
        }
        // if we are not lying down anymore, either standing up or falling, we do not execute any motion
        _ => {}
    }

    // Update completed status based on motion activity
    if !keyframe_executor.is_motion_active() && completed.is_none() {
        *completed = Some(Instant::now());
        return;
    }

    if let Some(start_time) = *completed {
        look_around(&mut nao_manager, start_time, 2.0, 1.0, 0.25);
        step_context.request_stand();
        if start_time.elapsed().as_secs() > 2 {
            // If the motion has been inactive for more than 2 seconds, we consider it completed
            standup.completed = true;
        }
    }
}

fn look_around(
    nao_manager: &mut NaoManager,
    starting_time: Instant,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = starting_time.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };

    nao_manager.set_head(
        position,
        HeadJoints::fill(NaoManager::HEAD_STIFFNESS),
        Priority::default(),
    );
}
