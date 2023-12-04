use enum_dispatch::enum_dispatch;
use miette::Result;
use nidhogg::NaoControlMessage;

use tyr::prelude::*;

use crate::{
    behavior::{
        behaviors::{Example, Initial},
        roles::{Keeper, Striker},
    },
    filter::button::HeadButtons,
    primary_state::PrimaryState,
};

/// Context that is passed into the behavior engine. It contains all necessary
/// information for executing behaviors and transitioning between different
/// behaviors
#[derive(Clone, Copy)]
pub struct Context<'a> {
    /// Primary state of the robot
    pub primary_state: &'a PrimaryState,
    /// State of the headbuttons of a robot
    pub head_buttons: &'a HeadButtons,
}

/// Trait that has be implemented for each behavior and what that behavior
/// does.
#[enum_dispatch]
pub trait Execute {
    /// Defines what the robot does when the corresponding behavior is executed.
    fn execute(
        &mut self,
        context: Context,
        current_role: &Role,
        control_message: &mut NaoControlMessage,
    );
}

/// Defines a behavior and a state for each behavior
#[enum_dispatch(Execute)]
pub enum Behavior {
    Initial(Initial),
    Example(Example),
    // Add new behaviors above
}

impl Default for Behavior {
    fn default() -> Self {
        Behavior::Initial(Initial)
    }
}

/// Trait that has to be implemented for each role and defines what behaviors
/// a robot with that role should perform.
#[enum_dispatch]
pub trait Transition {
    /// Defines the behavior transitions for a specific role.
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut Behavior,
    ) -> Behavior;
}

/// Defines a role and corresponding state
#[enum_dispatch(Transition)]
pub enum Role {
    Keeper(Keeper),
    Striker(Striker),
}

impl Role {
    /// Get the default role for each robot based on that robots player number
    fn by_player_number() -> Self {
        // TODO: get the default role for each robot by player number
        Role::Keeper(Keeper)
    }
}

/// Resource that is exposed and keeps track of the current role and behavior
pub struct Engine {
    /// Current robot role
    role: Role,
    /// Current robot behavior
    behavior: Behavior,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            role: Role::by_player_number(),
            behavior: Behavior::default(),
        }
    }
}

impl Engine {
    /// Assigns roles based on player number and other information like what
    /// robot is closest to the ball, missing robots, etc.
    fn assign_role(&self, _context: Context) -> Role {
        //TODO: assign roles based on robot player numbers and missing robots, etc.
        Role::by_player_number()
    }

    /// Executes one step of the behavior engine
    pub fn step(&mut self, context: Context, control_message: &mut NaoControlMessage) {
        self.role = self.assign_role(context);
        self.behavior = self.role.transition_behavior(context, &mut self.behavior);
        self.behavior.execute(context, &self.role, control_message);
    }
}

/// System that is called to execute one step of the behavior engine each cycle
#[system]
pub fn step(
    engine: &mut Engine,
    control_message: &mut NaoControlMessage,
    primary_state: &PrimaryState,
    head_buttons: &HeadButtons,
) -> Result<()> {
    let context = Context {
        primary_state,
        head_buttons,
    };

    engine.step(context, control_message);

    Ok(())
}

/// A module providing a state machine that keeps track of what behavior a
/// robot is doing.
///
/// Each behavior has an execute function that is called to
/// execute that behavior, this functionality can be implemented by
/// implementing the `Execute` trait.
///
/// Transitions between various behaviors are defined per role. New roles can
/// be defined by implementing the `Transition` trait.
///
/// This module provides the following resources to the application:
/// - [`Engine`]
pub struct BehaviorEngineModule;

impl Module for BehaviorEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app.init_resource::<Engine>()?.add_system(step))
    }
}
