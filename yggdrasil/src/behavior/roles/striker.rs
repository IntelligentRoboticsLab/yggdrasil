use crate::behavior::engine::{Behavior, Context, Role};

impl Role {
    pub fn striker_behaviour(&self, _ctx: &mut Context) -> Behavior {
        Behavior::initial()
    }
}
