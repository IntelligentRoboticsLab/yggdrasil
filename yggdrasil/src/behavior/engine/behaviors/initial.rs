use crate::behavior::{
    engine::{Behavior, BehaviorState, Context},
    Role,
};
use nidhogg::NaoControlMessage;

use super::Example;

#[derive(Copy, Clone, Debug, Default)]
pub struct Initial;

impl BehaviorState for Initial {
    fn execute(&mut self, _ctx: &mut Context, _control_message: &mut NaoControlMessage) {}

    fn transition(self, ctx: &Context) -> Behavior {
        match ctx {
            // do something if role is keeper
            Context {
                role: Role::Keeper, ..
            } => Behavior::Initial(Initial::default()),
            // do something with other role and game phase info
            Context {
                role, game_phase, ..
            } => {
                let _ = role;
                let _ = game_phase;
                Behavior::Example(Example::default())
            }
        }
    }
}
