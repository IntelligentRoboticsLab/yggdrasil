use crate::behaviour::behaviour_engine::engine::{Behaviour, BehaviourContext, BehaviourState};
use crate::game_phase::GamePhase;

pub fn transition_keeper_role_behaviour(
    behaviour: Behaviour,
    context: &BehaviourContext,
) -> Behaviour {
    use Behaviour::*;
    use GamePhase::*;
    match (behaviour, context.game_phase) {
        (InitialBehaviour(state), Normal) => ExampleBehaviour(state.into()),
        (InitialBehaviour(state), Timeout) => ExampleBehaviour(state.into()),
        _ => InitialBehaviour(BehaviourState::default()),
    }
}
