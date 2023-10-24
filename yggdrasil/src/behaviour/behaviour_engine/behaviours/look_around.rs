use nidhogg::NaoControlMessage;
use tyr::ResMut;

use crate::behaviour::behaviour_engine::engine::{ImplBehaviour, BehaviourContext};

#[derive(Hash, PartialEq, Eq)]
pub struct LookAroundState {
    test: i32,
}

// fn execute() -> NaoControlMessage {
//     NaoControlMessage::default()
// }

pub struct LookAround;

impl ImplBehaviour for LookAround {
    fn execute(&self, ctx: &mut BehaviourContext) -> ResMut<NaoControlMessage> {
        //ctx.ball_position
        //ctx.currntb = new LookAroundState{}
        ResMut{NaoControlMessage::default(); _}
    }
}

// pub fn look_around(ctx: &mut Context) ->Result<()> {
//     // ...
// }
