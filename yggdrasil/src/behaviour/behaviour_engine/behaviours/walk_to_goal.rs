use nidhogg::types::Color;
use nidhogg::NaoControlMessage;

use crate::behaviour::behaviour_engine::engine::{BehaviourContext, ImplBehaviour};

use crate::behaviour::Role::Keeper;

#[derive(Debug, Default)]
pub struct WalkToGoal {
    some_state: i8,
}

impl ImplBehaviour for WalkToGoal {
    fn execute(&mut self, context: &mut BehaviourContext, control_message: &mut NaoControlMessage) {
        if *context.role == Keeper {
            control_message.chest = Color {
                red: 255.0,
                green: 0.0,
                blue: 0.0,
            };
        };

        self.some_state += 1;
    }
}
