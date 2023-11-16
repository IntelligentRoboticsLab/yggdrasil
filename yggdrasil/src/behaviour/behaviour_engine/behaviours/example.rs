use nidhogg::types::Color;
use nidhogg::NaoControlMessage;

use crate::behaviour::behaviour_engine::engine::{BehaviourContext, ImplBehaviour};

use crate::behaviour::Role::Keeper;

#[derive(Debug, Default)]
pub struct ExampleBehaviour {
    some_state: i8,
}

impl ImplBehaviour for ExampleBehaviour {
    fn execute(&mut self, ctx: &mut BehaviourContext, ctrl_message: &mut NaoControlMessage) {
        if *ctx.role == Keeper {
            ctrl_message.chest = Color {
                red: 255.0,
                green: 0.0,
                blue: 0.0,
            };
        };

        self.some_state += 1;
    }
}
