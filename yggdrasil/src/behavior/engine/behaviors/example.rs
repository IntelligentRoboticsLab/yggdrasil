use crate::behavior::engine::{Behavior, BehaviorState, Context};
use crate::game_phase::GamePhase;
use nidhogg::NaoControlMessage;

use super::initial::Initial;

#[derive(Copy, Clone, Debug, Default)]
pub struct Example;

impl BehaviorState for Example {
    fn execute(&mut self, _ctx: &mut Context, _control_message: &mut NaoControlMessage) {}

    fn transition(self, ctx: &Context) -> Behavior {
        match ctx.game_phase {
            GamePhase::Timeout => Behavior::Example(Example::default()),
            _ => Behavior::Initial(Initial::default()),
        }
    }
}
