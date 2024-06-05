use crate::{
    behavior::engine::{Behavior, Context},
    motion::walk::engine::WalkingEngine,
    motion::{motion_manager::MotionManager, motion_types::MotionType, step_planner::StepPlanner},
    nao::manager::{NaoManager, Priority},
    sensor::falling::{FallState, LyingDirection},
};

/// Behavior dedicated to handling the getup sequence of the robot.
/// The behavior will be entered once the robot is confirmed to be lying down,
/// this will execute the appropriate standup motion after which the robot will return
/// to the appropriate next behavior.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Standup;

impl Behavior for Standup {
    fn execute(
        &mut self,
        context: Context,
        _nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        motion_manager: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) {
        // check the direction the robot is lying and execute the appropriate motion
        match context.fall_state {
            FallState::Lying(LyingDirection::FacingDown) => {
                motion_manager.start_new_motion(MotionType::StandupStomach, Priority::High);
            }
            FallState::Lying(LyingDirection::FacingUp) => {
                motion_manager.start_new_motion(MotionType::StandupBack, Priority::High);
            }
            // if we are not lying down anymore, either standing up or falling, we do not execute any motion
            _ => {}
        }
    }
}
