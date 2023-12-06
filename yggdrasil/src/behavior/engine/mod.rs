pub mod behaviors;

use enum_dispatch::enum_dispatch;
use miette::Result;
use nidhogg::NaoControlMessage;

use tyr::prelude::*;

use crate::{
    behavior::{PrimaryState, Role},
    game_phase::GamePhase,
};

use self::behaviors::{Example, Initial};

pub struct Context<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

#[enum_dispatch]
pub trait BehaviorState {
    fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage);
    fn transition(self, ctx: &Context) -> Behavior;
}

#[derive(Copy, Clone)]
#[enum_dispatch(BehaviorState)]
pub enum Behavior {
    Initial(Initial),
    Example(Example),
}

// enum_dispatch macro generates this for us :D

// impl Behave for Behavior {
//     fn transition(self, context: &Context) -> Self {
//         match self {
//             Behavior::Initial(state) => state.transition(context),
//             Behavior::Example(state) => state.transition(context),
//         }
//     }

//     fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
//         match self {
//             Behavior::Initial(state) => state.execute(ctx, control_message),
//             Behavior::Example(state) => state.execute(ctx, control_message),
//         }
//     }
// }

impl Behavior {
    fn initial() -> Self {
        Behavior::Initial(Initial)
    }
}

pub struct Engine {
    current_behavior: Behavior,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            current_behavior: Behavior::initial(),
        }
    }
}

impl Engine {
    pub fn step(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
        self.current_behavior.execute(ctx, control_message);
        self.current_behavior = self.current_behavior.transition(ctx);
    }
}

#[system]
pub fn step(
    engine: &mut Engine,
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

pub struct BehaviorEngineModule;

impl Module for BehaviorEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app.init_resource::<Engine>()?.add_system(step))
    }
}
