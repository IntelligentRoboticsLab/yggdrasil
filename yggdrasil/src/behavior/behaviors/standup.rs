use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::keyframe::{KeyframeExecutor, MotionType},
    nao::Priority,
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
    standup.completed = !keyframe_executor.is_motion_active();
}
