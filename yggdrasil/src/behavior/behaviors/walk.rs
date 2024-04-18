use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context},
    motion::step_planning::StepPlanner,
    nao::manager::NaoManager,
    walk::engine::{Step, WalkingEngine},
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
        walking_engine: &mut WalkingEngine,
        step_planner: &mut StepPlanner,
    ) {
        step_planner.set_target(Point2::new(0., 0.));
        // walking_engine.request_walk(self.step);
    }
}
