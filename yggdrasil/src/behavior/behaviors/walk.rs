use crate::{
    behavior::engine::{Behavior, Context},
    motion::motion_manager::MotionManager,
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
        _motion_manager: &mut MotionManager,
    ) {
        walking_engine.request_walk(self.step);
    }
}
