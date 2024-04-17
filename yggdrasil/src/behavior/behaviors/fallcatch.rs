use crate::{
    behavior::engine::{Behavior, Context},
    motion::motion_manager::MotionManager,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FallCatch;

impl Behavior for FallCatch {
    fn execute(
        &mut self,
        _: Context,
        nao_manager: &mut NaoManager,
        _: &mut WalkingEngine,
        _: &mut MotionManager,
    ) {
        nao_manager.unstiff_legs(Priority::Critical);
        nao_manager.unstiff_arms(Priority::Critical);
        nao_manager.unstiff_head(Priority::Critical);
    }
}
