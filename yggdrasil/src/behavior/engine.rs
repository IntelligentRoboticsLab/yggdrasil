//! The engine managing behavior execution and role state.

use bifrost::communication::GameControllerMessage;
use enum_dispatch::enum_dispatch;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{CatchFall, Observe, Standup, StartUp, Unstiff, Walk},
        primary_state::PrimaryState,
        roles::Attacker,
        BehaviorConfig,
    },
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig, yggdrasil::YggdrasilConfig},
        debug::DebugContext,
    },
    game_controller::GameControllerConfig,
    localization::RobotPose,
    motion::{keyframe::KeyframeExecutor, step_planner::StepPlanner, walk::engine::WalkingEngine},
    nao::{manager::NaoManager, RobotInfo},
    prelude::*,
    sensor::{
        button::{ChestButton, HeadButtons},
        falling::FallState,
        fsr::Contacts,
    },
    vision::ball_detection::classifier::Balls,
};

use super::{
    behaviors::{Stand, StandLookAt, WalkToSet},
    roles::Keeper,
};

/// Context that is passed into the behavior engine.
///
/// It contains all necessary information for executing behaviors and
/// transitioning between different behaviors.
#[derive(Clone)]
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
    /// Config containing information by which the player can be identified
    pub player_config: &'a PlayerConfig,
    /// Config containing information about the layout of the field.
    pub layout_config: &'a LayoutConfig,
    /// Config containing general information
    pub yggdrasil_config: &'a YggdrasilConfig,
    /// Config containing parameters for various behaviors
    pub behavior_config: &'a BehaviorConfig,
    /// Contains the message received from the game-controller.
    pub game_controller_message: Option<&'a GameControllerMessage>,
    /// Contains the game-controller config.
    pub game_controller_config: &'a GameControllerConfig,
    /// Contains information of the current Falling state of the robot
    pub fall_state: &'a FallState,
    /// Contains the pose of the robot.
    pub pose: &'a RobotPose,
    /// Contains the ball position.
    pub ball_position: &'a Option<Point2<f32>>,
    /// Contains the current behavior.
    pub current_behavior: BehaviorKind,
}

/// Control that is passed into the behavior engine.
///
/// It contains all necessary robot control for executing behaviors.
pub struct Control<'a> {
    pub nao_manager: &'a mut NaoManager,
    pub walking_engine: &'a mut WalkingEngine,
    pub keyframe_executor: &'a mut KeyframeExecutor,
    pub step_planner: &'a mut StepPlanner,
    pub debug_context: &'a mut DebugContext,
}

/// A trait representing a behavior that can be performed.
///
/// It is used to define the actions the robot will take when the corresponding behavior is executed.
/// The behavior is dependent on the current context, and any control messages.
///
/// # Examples
/// ```
/// use yggdrasil::behavior::engine::{Behavior, Context, Control};
///
/// struct Dance;
///
/// impl Behavior for Dance {
///     fn execute(
///         &mut self,
///         context: Context,
///         control: &mut Control,
///     ) {
///         // Dance like nobody's watching 🕺!
///     }
/// }
/// ```
// This trait is marked with `#[enum_dispatch]` to reduce boilerplate when adding new behaviors
#[enum_dispatch]
pub trait Behavior {
    /// Defines what the robot does when the corresponding behavior is executed.
    fn execute(&mut self, context: Context, control: &mut Control);
}

/// An enum containing the possible behaviors for a robot.
///
/// Each variant of this enum corresponds to a specific behavior and its associated
/// state.
/// The actual behavior is defined by implementing the [`Behavior`] trait for the state of each variant.
///
/// # Notes
/// - New behavior implementations should be added as new variants to this enum.
/// - The specific struct for each behavior (e.g., [`Stand`], [`StartUp`]) should implement the [`Behavior`] trait.
#[enum_dispatch(Behavior)]
#[derive(Debug, PartialEq, Clone)]
pub enum BehaviorKind {
    StartUp(StartUp),
    Unstiff(Unstiff),
    StandLookAt(StandLookAt),
    Observe(Observe),
    Stand(Stand),
    Walk(Walk),
    WalkToSet(WalkToSet),
    Standup(Standup),
    CatchFall(CatchFall),
    // Add new behaviors here!
}

impl Default for BehaviorKind {
    fn default() -> Self {
        BehaviorKind::Stand(Stand)
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
///     behaviors::Unstiff,
///     engine::{BehaviorKind, Context, Control, Role},
/// };
///
/// struct SecretAgent;
///
/// impl Role for SecretAgent {
///     fn transition_behavior(
///         &mut self,
///         context: Context,
///         control: &mut Control,
///     ) -> BehaviorKind {
///         // Implement behavior transitions for secret agent 🕵️
///         // E.g. Disguise -> Assassinate
///         BehaviorKind::Unstiff(Unstiff::default())
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
    fn transition_behavior(&mut self, context: Context, control: &mut Control) -> BehaviorKind;
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
/// - The specific struct for each role (e.g., [`Attacker`]) should implement the [`Role`] trait.
#[enum_dispatch(Role)]
pub enum RoleKind {
    Attacker(Attacker),
    // Add new roles here!
    Keeper(Keeper),
}

impl RoleKind {
    /// Get the default role for each robot based on that robots player number
    fn by_player_number(player_number: u8) -> Self {
        // TODO: get the default role for each robot by player number
        match player_number {
            1 => RoleKind::Keeper(Keeper),
            5 => RoleKind::Attacker(Attacker),
            _ => RoleKind::Attacker(Attacker),
        }
    }
}

/// Resource that is exposed and keeps track of the current role and behavior.
pub struct Engine {
    /// Current robot role
    role: RoleKind,
    /// Current robot behavior
    // TODO: Make private.
    pub behavior: BehaviorKind,
    pub prev_behavior_for_standup: Option<BehaviorKind>,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            role: RoleKind::Attacker(Attacker),
            behavior: BehaviorKind::default(),
            prev_behavior_for_standup: None,
        }
    }
}

impl Engine {
    /// Assigns roles based on player number and other information like what
    /// robot is closest to the ball, missing robots, etc.
    fn assign_role(&self, context: Context) -> RoleKind {
        RoleKind::by_player_number(context.player_config.player_number)
    }

    /// Executes one step of the behavior engine
    pub fn step(&mut self, context: Context, control: &mut Control) {
        self.role = self.assign_role(context.clone());

        self.transition(context.clone(), control);

        self.behavior.execute(context.clone(), control);
    }

    pub fn transition(&mut self, context: Context, control: &mut Control) {
        if let BehaviorKind::StartUp(_) = self.behavior {
            if control.walking_engine.is_sitting() || context.head_buttons.all_pressed() {
                self.behavior = BehaviorKind::Unstiff(Unstiff);
            }
            if *context.primary_state == PrimaryState::Initial {
                self.behavior = BehaviorKind::Stand(Stand);
            }
            return;
        }

        // unstiff has the number 1 precedence
        if let PrimaryState::Unstiff = context.primary_state {
            self.behavior = BehaviorKind::Unstiff(Unstiff);
            return;
        }

        if let BehaviorKind::Standup(standup) = self.behavior {
            if standup.completed() {
                self.behavior = self.prev_behavior_for_standup.take().unwrap();
            }
            return;
        }

        // next up, damage prevention and standup motion take precedence
        match context.fall_state {
            FallState::Lying(_) => {
                self.behavior = BehaviorKind::Standup(Standup::default());
                return;
            }
            FallState::Falling(_) => {
                if !matches!(context.primary_state, PrimaryState::Penalized) {
                    if self.prev_behavior_for_standup.is_none() {
                        self.prev_behavior_for_standup = Some(self.behavior.clone());
                    }
                    self.behavior = BehaviorKind::CatchFall(CatchFall);
                }
                return;
            }
            FallState::None => {
                if matches!(context.current_behavior, BehaviorKind::CatchFall(_)) {
                    self.behavior = self.prev_behavior_for_standup.take().unwrap();
                    return;
                }
            }
        }

        let ball_or_origin = context.ball_position.unwrap_or(Point2::origin());

        self.behavior = match context.primary_state {
            PrimaryState::Unstiff => BehaviorKind::Unstiff(Unstiff),
            PrimaryState::Standby
            | PrimaryState::Penalized
            | PrimaryState::Finished
            | PrimaryState::Calibration => BehaviorKind::Stand(Stand),
            PrimaryState::Initial => BehaviorKind::StandLookAt(StandLookAt {
                target: Point2::origin(),
            }),
            PrimaryState::Ready => BehaviorKind::WalkToSet(WalkToSet),
            PrimaryState::Set => BehaviorKind::StandLookAt(StandLookAt {
                target: ball_or_origin,
            }),
            PrimaryState::Playing { .. } => self.role.transition_behavior(context, control),
        };
    }
}

/// System that is called to execute one step of the behavior engine each cycle
#[system]
#[allow(clippy::type_complexity)]
pub fn step(
    (engine, primary_state): (&mut Engine, &mut PrimaryState),
    robot_info: &RobotInfo,
    (head_buttons, chest_button, contacts): (&HeadButtons, &ChestButton, &Contacts),
    (player_config, layout_config, yggdrasil_config, behavior_config, game_controller_config): (
        &PlayerConfig,
        &LayoutConfig,
        &YggdrasilConfig,
        &BehaviorConfig,
        &GameControllerConfig,
    ),
    (nao_manager, walking_engine, keyframe_executor, step_planner, debug_context): (
        &mut NaoManager,
        &mut WalkingEngine,
        &mut KeyframeExecutor,
        &mut StepPlanner,
        &mut DebugContext,
    ),
    game_controller_message: &Option<GameControllerMessage>,
    (robot_pose, balls, fall_state): (&RobotPose, &Balls, &FallState),
) -> Result<()> {
    let context = Context {
        robot_info,
        primary_state,
        head_buttons,
        chest_button,
        contacts,
        player_config,
        layout_config,
        yggdrasil_config,
        behavior_config,
        game_controller_message: game_controller_message.as_ref(),
        game_controller_config,
        fall_state,
        pose: robot_pose,
        ball_position: &balls.most_confident_ball().map(|b| b.position),
        current_behavior: engine.behavior.clone(),
    };

    let mut control = Control {
        nao_manager,
        walking_engine,
        keyframe_executor,
        step_planner,
        debug_context,
    };

    engine.step(context, &mut control);

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
            .add_staged_system(SystemStage::Init, step))
    }
}
