use crate::behaviour::behaviour_engine::engine::{Behaviour, BehaviourContext, BehaviourState};
use crate::game_phase::GamePhase;

pub fn transition_keeper_role_behaviour(state: Behaviour, context: &BehaviourContext) -> Behaviour {
    use Behaviour as B;
    match (state, context.game_phase) {
        (B::InitialBehaviour(state), GamePhase::Normal) => B::ExampleBehaviour(state.into()),
        (B::InitialBehaviour(state), GamePhase::Timeout) => B::ExampleBehaviour(state.into()),
        _ => B::InitialBehaviour(BehaviourState::default()),
    }
}
