use enum_dispatch::enum_dispatch;
use miette::Result;
use nidhogg::NaoControlMessage;

use tyr::prelude::*;

use crate::{
    behavior::{
        behaviors::{Example, Initial},
        roles::{Keeper, Striker},
    },
    game_phase::GamePhase,
    primary_state::PrimaryState,
};

#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
}

#[enum_dispatch]
pub trait Execute {
    fn execute(
        &mut self,
        ctx: Context,
        current_role: &Role,
        control_message: &mut NaoControlMessage,
    );
}

#[enum_dispatch(Execute)]
pub enum Behavior {
    Initial(Initial),
    Example(Example),
}

impl Default for Behavior {
    fn default() -> Self {
        Behavior::Initial(Initial)
    }
}

#[enum_dispatch]
pub trait Transition {
    fn transition_behavior(&mut self, ctx: Context, current_behavior: &mut Behavior) -> Behavior;
}

#[enum_dispatch(Transition)]
pub enum Role {
    Keeper(Keeper),
    Striker(Striker),
}

impl Default for Role {
    fn default() -> Self {
        //TODO assign roles
        Role::Keeper(Keeper {})
    }
}

#[derive(Default)]
pub struct Engine {
    role: Role,
    behavior: Behavior,
}

impl Engine {
    fn assign_role(&self, _ctx: Context) -> Role {
        //TODO assign roles
        Role::default()
    }

    pub fn step(&mut self, ctx: Context, control_message: &mut NaoControlMessage) {
        self.role = self.assign_role(ctx);
        self.behavior = self.role.transition_behavior(ctx, &mut self.behavior);
        self.behavior.execute(ctx, &self.role, control_message);
    }
}

#[system]
pub fn step(
    engine: &mut Engine,
    control_message: &mut NaoControlMessage,
    game_phase: &GamePhase,
    primary_state: &PrimaryState,
) -> Result<()> {
    let context = Context {
        primary_state,
        game_phase,
    };

    engine.step(context, control_message);

    Ok(())
}

pub struct BehaviorEngineModule;

impl Module for BehaviorEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app.init_resource::<Engine>()?.add_system(step))
    }
}
