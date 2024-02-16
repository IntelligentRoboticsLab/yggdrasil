use crate::behavior::engine::{Behavior, Context};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl Behavior for Initial {
    fn execute(&mut self, _context: Context, _control_message: &mut NaoControlMessage) {}
}
