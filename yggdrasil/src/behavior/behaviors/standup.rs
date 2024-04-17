use std::time::Instant;

use crate::{
    behavior::engine::{Behavior, Context},
    filter::falling::{FallState, LyingDirection},
    motion::{motion_manager::MotionManager, motion_types::MotionType},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Standup {
    pub standup_starting_time: Instant,
}

impl Default for Standup {
    fn default() -> Self {
        Standup {
            standup_starting_time: Instant::now(),
        }
    }
}

impl Behavior for Standup {
    fn execute(
        &mut self,
        context: Context,
        _: &mut NaoManager,
        _: &mut WalkingEngine,
        motion_manager: &mut MotionManager,
    ) {
        println!("Standup Behavior");
        // check the direction the robot is lying and execute the appropriate motion
        match context.fall_state {
            FallState::InStandup => {
                // if we are still in standup, we simply exit out of the execution
                return;
            }
            FallState::Lying(LyingDirection::FacingDown) => {
                motion_manager.start_new_motion(MotionType::StandupStomach, Priority::High)
            }
            FallState::Lying(LyingDirection::FacingUp) => {
                motion_manager.start_new_motion(MotionType::StandupBack, Priority::High)
            }
            _ => {}
        }
    }
}
