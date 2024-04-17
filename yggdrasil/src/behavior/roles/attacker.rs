use crate::{
    behavior::{
        behaviors::Walk,
        engine::{BehaviorKind, Context, Role},
    },
    walk::engine::{Step, WalkingEngine},
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
        _walking_engine: &mut WalkingEngine,
    ) -> BehaviorKind {
        BehaviorKind::Walk(Walk {
            step: Step {
                forward: 0.04,
                left: 0.0,
                turn: 0.0,
            },
        })
    }
}
