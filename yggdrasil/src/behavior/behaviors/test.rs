use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Test;

impl Behavior for Test {
    fn execute(
        &mut self,
        _context: Context,
        nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
    ) {
        nao_manager.set_head(HeadJoints::fill(0.), HeadJoints::fill(0.3), Priority::Low);
    }
}
