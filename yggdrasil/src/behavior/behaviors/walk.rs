use crate::{
    behavior::engine::{Behavior, Context},
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
    ) {
        walking_engine.request_walk(Step {
            forward: 0.04,
            left: 0.0,
            turn: 0.0,
        });
    }
}
