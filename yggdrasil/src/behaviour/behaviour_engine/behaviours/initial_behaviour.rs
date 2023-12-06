use crate::behaviour::behaviour_engine::{
    engine::{Behave, Behaviour},
    Context,
};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl Behave for Initial {
    fn transition(self, _ctx: &Context) -> Behaviour {
        Behaviour::Initial(Initial)
    }

    fn execute(&mut self, _context: &mut Context, _control_message: &mut NaoControlMessage) {}
}
