use crate::{
    behaviour::{PrimaryState, Role},
    game_phase::GamePhase,
};

use enum_dispatch::enum_dispatch;
use miette::Result;
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

use crate::behaviour::behaviour_engine::behaviours::*;

pub struct Context<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

#[enum_dispatch]
pub trait Behave {
    fn transition(self, ctx: &Context) -> Behaviour;
    fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage);
}

#[derive(Copy, Clone)]
#[enum_dispatch(Behave)]
pub enum Behaviour {
    Initial(Initial),
    Example(Example),
}

impl Default for Behaviour {
    fn default() -> Self {
        Behaviour::Initial(Initial::default())
    }
}

// enum_dispatch crate doet dit voor ons

// impl Behave for Behaviour {
//     fn transition(self, context: &Context) -> Self {
//         match self {
//             Behaviour::Initial(state) => state.transition(context),
//             Behaviour::Example(state) => state.transition(context),
//         }
//     }

//     fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
//         match self {
//             Behaviour::Initial(state) => state.execute(ctx, control_message),
//             Behaviour::Example(state) => state.execute(ctx, control_message),
//         }
//     }
// }

#[derive(Default)]
pub struct BehaviourEngine {
    current_behaviour: Behaviour,
}

impl BehaviourEngine {
    pub fn step(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
        self.current_behaviour.execute(ctx, control_message);
        self.current_behaviour = self.current_behaviour.transition(ctx);
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
    let mut context = Context {
        primary_state,
        game_phase,
        role,
    };

    engine.step(&mut context, control_message);

    Ok(())
}
