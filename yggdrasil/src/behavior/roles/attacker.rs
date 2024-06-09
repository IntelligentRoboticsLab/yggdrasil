use crate::{
    behavior::{
        behaviors::Walk,
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::walk::engine::Step,
};

pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(&mut self, _context: Context, _control: &mut Control) -> BehaviorKind {
        BehaviorKind::Walk(Walk {
            step: Step {
                forward: 0.04,
                left: 0.0,
                turn: 0.0,
            },
        })
    }
}
