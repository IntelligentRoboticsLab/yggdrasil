use crate::behavior::engine::{BehaviorKind, Context, Role};

pub struct Striker;

impl Role for Striker {
    fn transition_behavior(
        &mut self,
        _context: Context,
        _current_behavior: &mut BehaviorKind,
    ) -> BehaviorKind {
        BehaviorKind::default()
    }
}
