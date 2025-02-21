use bevy::prelude::*;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::keyframe::{AnimationManager, MotionType},
    sensor::falling::{FallState, LyingDirection},
};

/// Behavior dedicated to handling the getup sequence of the robot.
/// The behavior will be entered once the robot is confirmed to be lying down,
/// this will execute the appropriate standup motion after which the robot will return
/// to the appropriate next behavior.
///
/// # Notes
/// - Currently, the direction is simply specified by a bool, since we have 2 getup types.
///   However, in the future this could be expanded
///
#[derive(Resource, Default)]
pub struct Standup {
    completed: bool,
    standup_type: Option<MotionType>,
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

pub fn standup(
    mut standup: ResMut<Standup>,
    fall_state: Res<FallState>,
    mut animation_manager: ResMut<AnimationManager>,
) {
    if standup.standup_type.is_none() {
        match fall_state.as_ref() {
            FallState::Lying(LyingDirection::FacingDown) => {
                standup.standup_type = Some(MotionType::StandupStomach);
            }
            FallState::Lying(LyingDirection::FacingUp) => {
                standup.standup_type = Some(MotionType::StandupBack);
            }
            _ => {
                // we will simply exit the standup behaviour, if the fallstate doesnt agree
                standup.completed = true;
                return;
            }
        }
    }

    // check the direction the robot is lying and execute the appropriate motion
    match fall_state.as_ref() {
        FallState::Lying(LyingDirection::FacingDown) => {
            animation_manager.start_new_motion(MotionType::StandupStomach, false);
        }
        FallState::Lying(LyingDirection::FacingUp) => {
            animation_manager.start_new_motion(MotionType::StandupBack, false);
        }
        // if we are not lying down anymore, either standing up or falling, we do not execute any motion
        _ => {}
    }

    standup.completed = !animation_manager.is_motion_active();
}
