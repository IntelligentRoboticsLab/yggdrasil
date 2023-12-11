use crate::behavior::{engine::Transition, Behavior, Context};

pub struct Keeper;

impl Transition for Keeper {
    fn transition_behavior(&mut self, _ctx: Context, _current_behavior: &mut Behavior) -> Behavior {
        todo!()
    }
}
