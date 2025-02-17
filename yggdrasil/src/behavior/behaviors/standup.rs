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

    // Issues:
    // - Important to use existing code from the keyframe manager for keyframe movement, however,
    //   currently too ingrained into the keyframe manager (impossible to be used outside)
    // - It's best to generalize most functions to all standup motions,
    //   however, functionalities like branching paths and exit routines are not possible in the usual framework
    //   Possible solution: New motion format for the standup motion (But maybe it'll get bloated again)
    // -

    // During all movements :
    //  -> Double bounded:
    //      -> firstly we have the recorrection bound: within this bound we can try a recorrection (move key joints)
    //      -> Secondly, fallcatch bound: when the robot enters these bounds, it's a point of no return (catch the fall)

    // During Transitions:
    //  -> Stable Waittime: Simply wait with any further movement, till the robot is deemed stable
    //  -> FailRoutines: More advanced routines the robot can go about to prevent damage and raise success chance
    //  -> (WIP) Branching: Brannching paths for the standup motion

    // Update completed status based on motion activity (TODO change this)

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
