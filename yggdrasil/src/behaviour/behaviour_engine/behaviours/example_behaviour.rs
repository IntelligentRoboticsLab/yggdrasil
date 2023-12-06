use crate::behaviour::behaviour_engine::behaviours::*;
use crate::behaviour::behaviour_engine::engine::{Behave, Behaviour};
use crate::behaviour::behaviour_engine::Context;
use crate::game_phase::GamePhase;
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone, Debug, Default)]
pub struct Example;

impl Behave for Example {
    fn transition(self, ctx: &Context) -> Behaviour {
        match ctx.game_phase {
            GamePhase::Timeout => Behaviour::Example(Example::default()),
            _ => Behaviour::Initial(Initial::default()),
        }
    }

    fn execute(&mut self, _context: &mut Context, _control_message: &mut NaoControlMessage) {}
}
