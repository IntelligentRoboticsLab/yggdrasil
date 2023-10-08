use nidhogg::NaoControlMessage;

use crate::behaviour::behaviour_engine::engine::ImplBehaviour;

pub struct LookAround;

impl ImplBehaviour for LookAround {
    fn execute(&self) -> NaoControlMessage {
        NaoControlMessage::default()
    }
}
