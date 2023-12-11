use crate::behavior::engine::{Context, Execute, Role};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone, Debug, Default)]
pub struct Example;

impl Execute for Example {
    fn execute(
        &mut self,
        _ctx: Context,
        _current_role: &Role,
        _control_message: &mut NaoControlMessage,
    ) {
    }
}
