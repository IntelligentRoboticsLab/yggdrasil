use crate::behaviour::behaviour_engine::{
    behaviours::WalkToGoal,
    engine::{Behaviour, BehaviourEngine, TransitionContext},
};
use crate::game_phase::GamePhase;

pub fn transition_keeper_role_behaviour(engine: &mut BehaviourEngine, context: &TransitionContext) {
    let new_behaviour = match *context.game_phase {
        GamePhase::Normal => Behaviour::WalkToGoal(WalkToGoal::default()),
        _ => Behaviour::None,
    };

    engine.transition(new_behaviour);
}
