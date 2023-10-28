use nidhogg::NaoControlMessage;
use nidhogg::types::Color;

use crate::behaviour::behaviour_engine::engine::{ImplBehaviour, BehaviourContext};

use crate::behaviour::Role::*;

#[derive(Hash, PartialEq, Eq)]
pub struct LookAroundState {
    test: i8,
}

// fn execute() -> NaoControlMessage {
//     NaoControlMessage::default()
// }

pub struct LookAround;

impl ImplBehaviour for LookAround {
    fn execute(&self, ctx: &mut BehaviourContext) -> NaoControlMessage {
        //ctx.ball_position
        let mut message = NaoControlMessage::default();

        if ctx.role == Keeper {
            message.chest = Color {red: 255.0, green: 0.0, blue: 0.0};
        };

        message
    }
}

// pub fn look_around(ctx: &mut Context) ->Result<()> {
//     // ...
// }
