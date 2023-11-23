use crate::behaviour::behaviour_engine::{BehaviourContext, BehaviourState, ImplBehaviour};
use nidhogg::NaoControlMessage;

#[derive(Copy, Clone)]
pub struct InitialBehaviour {}

impl InitialBehaviour {
    fn new() -> Self {
        InitialBehaviour {}
    }
}

impl Default for BehaviourState<InitialBehaviour> {
    fn default() -> Self {
        BehaviourState {
            state: InitialBehaviour::new(),
        }
    }
}

impl ImplBehaviour for BehaviourState<InitialBehaviour> {
    fn execute(
        &mut self,
        _context: &mut BehaviourContext,
        _control_message: &mut NaoControlMessage,
    ) {
    }
}
