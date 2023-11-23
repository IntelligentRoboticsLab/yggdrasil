use crate::behaviour::behaviour_engine::behaviours::*;
use crate::behaviour::behaviour_engine::{BehaviourContext, BehaviourState, ImplBehaviour};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone)]
pub struct ExampleBehaviour {}

impl From<BehaviourState<InitialBehaviour>> for BehaviourState<ExampleBehaviour> {
    fn from(_value: BehaviourState<InitialBehaviour>) -> Self {
        BehaviourState {
            state: ExampleBehaviour {},
        }
    }
}

impl ImplBehaviour for BehaviourState<ExampleBehaviour> {
    fn execute(
        &mut self,
        _context: &mut BehaviourContext,
        _control_message: &mut NaoControlMessage,
    ) {
    }
}
