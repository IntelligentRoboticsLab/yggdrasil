use crate::behaviour::behaviour_engine::{
    behaviours::ExampleBehaviour,
    engine::{Behaviour, BehaviourEngine, TransitionContext},
};
use crate::game_phase::GamePhase;

pub fn update_example_role_behaviour(engine: &mut BehaviourEngine, ctx: &TransitionContext) {
    let new_behaviour = match *ctx.game_phase {
        GamePhase::Normal => Behaviour::Example(ExampleBehaviour::default()),
        _ => Behaviour::None,
    };

    engine.transition(new_behaviour);
}
