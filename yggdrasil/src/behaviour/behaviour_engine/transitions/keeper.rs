use crate::behaviour::behaviour_engine::engine::{Behaviour, BehaviourContext, BehaviourState};
use crate::game_phase::GamePhase;

pub fn transition_keeper_role_behaviour(
    behaviour: Behaviour,
    context: &BehaviourContext,
) -> Behaviour {
    use Behaviour as B;
    use GamePhase as G;
    match (behaviour, context.game_phase) {
        (B::InitialBehaviour(state), G::Normal) => B::ExampleBehaviour(state.into()),
        (B::InitialBehaviour(state), G::Timeout) => B::ExampleBehaviour(state.into()),
        _ => B::InitialBehaviour(BehaviourState::default()),
    }
}
