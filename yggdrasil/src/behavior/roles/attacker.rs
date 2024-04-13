use crate::{
    behavior::{
        behaviors::Observe,
        engine::{BehaviorKind, Context, Role},
    },
    walk::engine::WalkingEngine,
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
    ) -> BehaviorKind {
        BehaviorKind::Observe(Observe::default())
    }
}
