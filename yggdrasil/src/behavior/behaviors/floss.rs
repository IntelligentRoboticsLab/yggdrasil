use crate::{
    behavior::engine::{Behavior, Context},
    motion::{motion_manager::MotionManager, motion_types::MotionType, step_planner::StepPlanner},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Floss;

impl Behavior for Floss {
    fn execute(
        &mut self,
        _context: Context,
        _nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        motion_manager: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) {
        if motion_manager.is_motion_active() {
            return;
        }
        motion_manager.start_new_motion(MotionType::Floss, Priority::High);
    }
}
