use crate::behaviour::behaviour_engine::{
    behaviours::WalkToGoal,
    engine::{Behaviour, BehaviourEngine, TransitionContext},
};
use crate::game_phase::GamePhase;

pub fn update_keeper_role_behaviour(engine: &mut BehaviourEngine, ctx: &TransitionContext) {
    let new_behaviour = match *ctx.game_phase {
        GamePhase::Normal => Behaviour::WalkToGoal(WalkToGoal::default()),
        _ => Behaviour::None,
    };

    engine.transition(new_behaviour);
}
