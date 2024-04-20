use crate::{
    behavior::{
        behaviors::Floss,
        engine::{BehaviorKind, Context, Role},
    },
    motion::motion_manager::MotionManager,
    walk::engine::WalkingEngine,
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
        _motion_manager: &mut MotionManager,
    ) -> BehaviorKind {
        BehaviorKind::Floss(Floss)
    }
}
