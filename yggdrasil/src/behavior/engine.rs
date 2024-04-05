//! The engine managing behavior execution and role state.

use enum_dispatch::enum_dispatch;

use crate::{
    behavior::{
        behaviors::{Initial, Passive},
        roles::Base,
        BehaviorConfig,
    },
    config::{layout::LayoutConfig, yggdrasil::YggdrasilConfig},
    filter::{
        button::{ChestButton, HeadButtons},
        fsr::Contacts,
    },
    nao::{self, manager::NaoManager, RobotInfo},
    prelude::*,
    primary_state::PrimaryState,
};

use super::behaviors::Observe;

/// Context that is passed into the behavior engine.
///
/// It contains all necessary information for executing behaviors and
/// transitioning between different behaviors.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    /// Robot info
    pub robot_info: &'a RobotInfo,
    /// Primary state of the robot
    pub primary_state: &'a PrimaryState,
    /// State of the headbuttons of a robot
    pub head_buttons: &'a HeadButtons,
    /// State of the chest button of a robot
    pub chest_button: &'a ChestButton,
    /// Contains information on whether the nao is touching the ground
    pub contacts: &'a Contacts,
    /// Config containing information about the layout of the field
    pub layout_config: &'a LayoutConfig,
    /// Config containing general information
    pub yggdrasil_config: &'a YggdrasilConfig,
    /// Config containing parameters for various behaviors
    pub behavior_config: &'a BehaviorConfig,
}

/// A trait representing a behavior that can be performed.
///
/// It is used to define the actions the robot will take when the corresponding behavior is executed.
/// The behavior is dependent on the current context, and any control messages.
///
/// # Examples
/// ```
/// use yggdrasil::behavior::engine::{Behavior, Context};
/// use yggdrasil::nao::manager::NaoManager;
///
/// struct Dance;
///
/// impl Behavior for Dance {
///     fn execute(
///         &mut self,
///         context: Context,
///         nao_manager: &mut NaoManager,
///     ) {
///         // Dance like nobody's watching 🕺!
///     }
/// }
/// ```
// This trait is marked with `#[enum_dispatch]` to reduce boilerplate when adding new behaviors
#[enum_dispatch]
pub trait Behavior {
    /// Defines what the robot does when the corresponding behavior is executed.
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager);
}

/// An enum containing the possible behaviors for a robot.
///
/// Each variant of this enum corresponds to a specific behavior and its associated
/// state.
/// The actual behavior is defined by implementing the [`Behavior`] trait for the state of each variant.
///
/// # Notes
/// - New behavior implementations should be added as new variants to this enum.
/// - The specific struct for each behavior (e.g., [`Initial`], [`Passive`]) should implement the [`Behavior`] trait.
#[enum_dispatch(Behavior)]
#[derive(Debug)]
pub enum BehaviorKind {
    Passive(Passive),
    Initial(Initial),
    Observe(Observe),
    // Add new behaviors here!
}

impl Default for BehaviorKind {
    fn default() -> Self {
        BehaviorKind::Passive(Passive)
    }
}

/// A trait representing a role for the robot.
///
/// This trait must be implemented for each specific role.
/// It defines the set of behaviors and how transitions between these behaviors should be handled
/// based on the role.
///
/// # Examples
/// ```
/// use yggdrasil::behavior::{
///     behaviors::Initial,
///     engine::{BehaviorKind, Context, Role}
/// };
///
/// struct SecretAgent;
///
/// impl Role for SecretAgent {
///     fn transition_behavior(
///         &mut self,
///         context: Context,
///         current_behavior: &mut BehaviorKind,
///     ) -> BehaviorKind {
///         // Implement behavior transitions for secret agent 🕵️
///         // E.g. Disguise -> Assassinate
///         BehaviorKind::Initial(Initial::default())
///     }
/// }
/// ```
// This trait is marked with `#[enum_dispatch]` to reduce boilerplate when adding new roles
#[enum_dispatch]
pub trait Role {
    /// Defines the behavior transitions for a specific role.
    ///
    /// # Returns
    /// - Returns the [`BehaviorKind`] the robot should transition to.
    fn transition_behavior(
        &mut self,
        context: Context,
        current_behavior: &mut BehaviorKind,
    ) -> BehaviorKind;
}

/// An enum containing the possible roles for a robot.
///
/// Each variant of this enum corresponds to a specific role and its associated
/// state. The state is used to define the underlying behaviors for the role, and
/// transitions between various behaviors are handled by implementing the [`Role`]
/// trait for the state.
///
/// # Notes
/// - New role implementations should be added as new variants to this enum
/// - The specific struct for each role (e.g., [`Base`]) should implement the [`Role`] trait.
#[enum_dispatch(Role)]
pub enum RoleKind {
    Base(Base),
    // Add new roles here!
}

impl RoleKind {
    /// Get the default role for each robot based on that robots player number
    fn by_player_number() -> Self {
        // TODO: get the default role for each robot by player number
        RoleKind::Base(Base)
    }
}

/// Resource that is exposed and keeps track of the current role and behavior.
pub struct Engine {
    /// Current robot role
    role: RoleKind,
    /// Current robot behavior
    behavior: BehaviorKind,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            role: RoleKind::by_player_number(),
            behavior: BehaviorKind::default(),
        }
    }
}

impl Engine {
    /// Assigns roles based on player number and other information like what
    /// robot is closest to the ball, missing robots, etc.
    fn assign_role(&self, _context: Context) -> RoleKind {
        // TODO: assign roles based on robot player numbers and missing robots, etc.
        RoleKind::by_player_number()
    }

    /// Executes one step of the behavior engine
    pub fn step(&mut self, context: Context, nao_manager: &mut NaoManager) {
        self.role = self.assign_role(context);
        self.behavior = self.role.transition_behavior(context, &mut self.behavior);
        self.behavior.execute(context, nao_manager);
    }
}

/// System that is called to execute one step of the behavior engine each cycle
#[system]
#[allow(clippy::too_many_arguments)]
pub fn step(
    engine: &mut Engine,
    nao_manager: &mut NaoManager,
    robot_info: &RobotInfo,
    primary_state: &PrimaryState,
    head_buttons: &HeadButtons,
    chest_button: &ChestButton,
    contacts: &Contacts,
    layout_config: &LayoutConfig,
    yggdrasil_config: &YggdrasilConfig,
    behavior_config: &BehaviorConfig,
) -> Result<()> {
    let context = Context {
        robot_info,
        primary_state,
        head_buttons,
        chest_button,
        contacts,
        layout_config,
        yggdrasil_config,
        behavior_config,
    };

    engine.step(context, nao_manager);

    Ok(())
}

/// A module providing a state machine that keeps track of what behavior a
/// robot is doing.
///
/// Each behavior has an execute function that is called to
/// execute that behavior, this functionality can be implemented by
/// implementing the [`Behavior`] trait.
///
/// Transitions between various behaviors are defined per role. New roles can
/// be defined by implementing the [`Role`] trait.
///
/// This module provides the following resources to the application:
/// - [`Engine`]
pub struct BehaviorEngineModule;

impl Module for BehaviorEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app
            .init_resource::<Engine>()?
            .add_system(step.after(nao::write_hardware_info)))
    }
}
