use nalgebra::Point2;

use crate::{
    behavior::engine::{Behavior, Context},
    motion::step_planning::StepPlanner,
    nao::manager::NaoManager,
    walk::engine::WalkingEngine,
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct WalkTo {
    pub target: Point2<f32>,
}

impl Behavior for WalkTo {
    fn execute(
        &mut self,
        context: Context,
        _nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        step_planner: &mut StepPlanner,
    ) {
        // x is infront, y is to the left

        step_planner.set_target(self.target)
    }
}
