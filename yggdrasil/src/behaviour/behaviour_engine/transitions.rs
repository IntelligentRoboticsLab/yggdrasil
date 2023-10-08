use miette::Result;
use tyr::prelude::*;

use crate::{
    behaviour::{Behaviour, BehaviourEngine, Role},
    game_phase::GamePhase,
};

#[system]
pub fn transitions(
    role: &Role,
    engine: &mut BehaviourEngine,
    game_phase: &GamePhase,
    // TODO: add necessary information for all transitions.
) -> Result<()> {
    use Role::*;

    match *role {
        // TODO: add other roles.
        Keeper => update_keeper_behaviour(&mut engine, &game_phase),
    }

    Ok(())
}

fn update_keeper_behaviour(engine: &mut BehaviourEngine, game_phase: &GamePhase) {
    // This is just an example
    let new_behaviour = match *game_phase {
        GamePhase::Normal => Behaviour::Stand,
        _ => Behaviour::Stand,
    };

    engine.transition(new_behaviour);
}
