use crate::behavior::engine::{Behavior, Context, Transition};

pub struct Striker;

impl Transition for Striker {
    fn transition_behavior(&mut self, _ctx: Context, _current_behavior: &mut Behavior) -> Behavior {
        todo!()
    }
}
