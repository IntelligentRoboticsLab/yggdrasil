use crate::{
    behaviour::{PrimaryState, Role},
    game_phase::GamePhase,
};

use miette::Result;
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

use crate::behaviour::behaviour_engine::behaviours::*;
use crate::behaviour::behaviour_engine::transitions::*;

#[derive(Debug)]
pub enum Behaviour {
    Example(ExampleBehaviour),
    None,
}

#[derive(Debug)]
pub struct BehaviourEngine {
    current_behaviour: Behaviour,
}

impl Default for BehaviourEngine {
    fn default() -> Self {
        BehaviourEngine {
            current_behaviour: Behaviour::None,
        }
    }
}

impl BehaviourEngine {
    pub fn execute_current_behaviour(
        &mut self,
        ctx: &mut BehaviourContext,
        ctrl_msg: &mut NaoControlMessage,
    ) {
        //TODO: just use dynamic dispatch instead? Seems cleaner, then the entire match statement
        //is not necessary and we can just directly call behaviour.execute().
        use Behaviour::*;
        match self.current_behaviour {
            Example(ref mut behaviour) => behaviour.execute(ctx, ctrl_msg),
            None => (),
        }
    }

    pub fn transition(&mut self, behaviour_type: Behaviour) {
        self.current_behaviour = behaviour_type;
    }
}

#[derive(Debug)]
pub struct BehaviourContext<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

pub trait ImplBehaviour {
    fn execute(&mut self, ctx: &mut BehaviourContext, ctrl_msg: &mut NaoControlMessage);
}

#[system]
pub fn executor(
    engine: &mut BehaviourEngine,
    ctrl_msg: &mut NaoControlMessage,
    role: &Role,
    game_phase: &GamePhase,
    primary_state: &PrimaryState,
) -> Result<()> {
    let mut ctx = BehaviourContext {
        primary_state: &primary_state,
        game_phase: &game_phase,
        role: &role,
    };

    engine.execute_current_behaviour(&mut ctx, &mut ctrl_msg);

    Ok(())
}

#[derive(Debug)]
pub struct TransitionContext<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

#[system]
pub fn transition_behaviour(
    role: &Role,
    engine: &mut BehaviourEngine,
    game_phase: &GamePhase,
    primary_state: &PrimaryState,
) -> Result<()> {
    //TODO is there a better way to do this?
    let ctx = TransitionContext {
        role: &role,
        primary_state: &primary_state,
        game_phase: &game_phase,
    };

    use Role::*;
    match *role {
        ExampleRole => update_example_role_behaviour(&mut engine, &ctx),
    }

    Ok(())
}
