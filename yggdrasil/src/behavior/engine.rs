use enum_dispatch::enum_dispatch;
use miette::Result;
use nidhogg::NaoControlMessage;

use tyr::prelude::*;

use crate::{game_phase::GamePhase, primary_state::PrimaryState};

use crate::behavior::behaviors::{Example, Initial};

pub struct Context<'a> {
    pub game_phase: &'a GamePhase,
    pub primary_state: &'a PrimaryState,
    pub role: &'a Role,
}

#[enum_dispatch]
pub trait BehaviorState {
    fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage);
}

#[derive(Copy, Clone)]
#[enum_dispatch(BehaviorState)]
pub enum Behavior {
    Initial(Initial),
    Example(Example),
}

// enum_dispatch macro generates this for us :D

// impl Behave for Behavior {
//     fn execute(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
//         match self {
//             Behavior::Initial(state) => state.execute(ctx, control_message),
//             Behavior::Example(state) => state.execute(ctx, control_message),
//         }
//     }
// }

impl Behavior {
    pub fn initial() -> Self {
        Behavior::Initial(Initial)
    }
}

pub enum RoleType {
    Keeper,
    Striker,
}

pub struct Role {
    role_type: RoleType,
    behavior: Behavior,
}

impl Role {
    fn initial() -> Self {
        // Do this based on the player number
        return Role {
            role_type: RoleType::Keeper,
            behavior: Behavior::initial(),
        };
    }

    fn transition_behaviour(&mut self, ctx: &mut Context) -> Behavior {
        match self.role_type {
            RoleType::Keeper => self.keeper_behaviour(ctx),
            RoleType::Striker => self.striker_behaviour(ctx),
        }
    }
}

pub struct Engine {
    role: Role,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            role: Role::initial(),
        }
    }
}

impl Engine {
    fn assign_role(&self, _ctx: &Context) -> Role {
        //TODO assign roles
        return Role::initial();
    }

    pub fn step(&mut self, ctx: &mut Context, control_message: &mut NaoControlMessage) {
        self.role = self.assign_role(ctx);
        self.role.behavior = self.role.transition_behaviour(ctx);
        self.role.behavior.execute(ctx, control_message);
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
