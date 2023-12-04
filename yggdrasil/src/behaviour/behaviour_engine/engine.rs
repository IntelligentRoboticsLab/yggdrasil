use crate::{
    behaviour::{PrimaryState, Role},
    game_phase::GamePhase,
};

use miette::Result;
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

use crate::behaviour::behaviour_engine::{behaviours::*, transitions::*};

#[derive(Copy, Clone)]
pub struct BehaviourState<S: Copy> {
    pub state: S,
}

pub trait ImplBehaviour {
    fn execute(&mut self, context: &mut BehaviourContext, control_message: &mut NaoControlMessage);
}

pub struct BehaviourContext<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

#[derive(Copy, Clone)]
pub enum Behaviour {
    InitialBehaviour(BehaviourState<InitialBehaviour>),
    ExampleBehaviour(BehaviourState<ExampleBehaviour>),
}

impl Default for Behaviour {
    fn default() -> Self {
        Behaviour::InitialBehaviour(BehaviourState::<InitialBehaviour>::default())
    }
}

impl Behaviour {
    fn transition(self, context: &BehaviourContext) -> Self {
        use Role as R;
        match context.role {
            R::Keeper => transition_keeper_role_behaviour(self, context),
        }
    }
}

#[derive(Default)]
pub struct BehaviourEngine {
    current_behaviour: Behaviour,
}

impl BehaviourEngine {
    fn execute(&mut self, context: &mut BehaviourContext, control_message: &mut NaoControlMessage) {
        use Behaviour as B;
        match self.current_behaviour {
            B::InitialBehaviour(ref mut behaviour) => behaviour.execute(context, control_message),
            B::ExampleBehaviour(ref mut behaviour) => behaviour.execute(context, control_message),
        }
    }

    pub fn step(
        &mut self,
        context: &mut BehaviourContext,
        control_message: &mut NaoControlMessage,
    ) {
        self.execute(context, control_message);
        self.current_behaviour = self.current_behaviour.transition(context);
    }
}

#[system]
pub fn step(
    engine: &mut BehaviourEngine,
    control_message: &mut NaoControlMessage,
    role: &Role,
    game_phase: &GamePhase,
    primary_state: &PrimaryState,
) -> Result<()> {
    let mut context = BehaviourContext {
        primary_state,
        game_phase,
        role,
    };

    engine.step(&mut context, control_message);

    Ok(())
}
