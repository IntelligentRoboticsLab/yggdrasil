use std::time::Instant;

use crate::{
    behavior::engine::{Behavior, Context},
    filter::falling::{FallState, LyingDirection},
    motion::{motion_manager::MotionManager, motion_types::MotionType},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

#[derive(Copy, Clone, Debug)]
pub struct Fallcatch;

impl Behavior for FallCatch {
    fn execute(
        &mut self,
        context: Context,
        _: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        motion_manager: &mut MotionManager,
    ) {
        // stop the walking engine
        walking_engine.request_idle();

        // check the direction the robot is lying and execute the appropriate motion
        match context.fall_filter.state {
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
