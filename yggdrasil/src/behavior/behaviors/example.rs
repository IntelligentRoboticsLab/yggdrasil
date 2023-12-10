use crate::behavior::engine::{BehaviorState, Context};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone, Debug, Default)]
pub struct Example;

impl BehaviorState for Example {
    fn execute(&mut self, _ctx: &mut Context, _control_message: &mut NaoControlMessage) {}
}
