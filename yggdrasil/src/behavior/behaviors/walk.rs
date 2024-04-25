use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context},
    motion::motion_manager::MotionManager,
    motion::step_planner::StepPlanner,
    motion::walk::engine::{Step, WalkingEngine},
    nao::manager::NaoManager,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Walk {
    pub step: Step,
}

impl Behavior for Walk {
    fn execute(
        &mut self,
        _context: Context,
        _nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        _: &mut MotionManager,
        step_planner: &mut StepPlanner,
    ) {
        step_planner.set_absolute_target(Point2::new(0., 0.));
    }
}
