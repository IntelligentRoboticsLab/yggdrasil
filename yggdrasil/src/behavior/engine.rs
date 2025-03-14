use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase};
use heimdall::{Bottom, Top};

use ml::{
    prelude::{MlTaskCommandsExt, ModelExecutor},
    MlModel,
};
use nalgebra::Point2;

use crate::{
    core::config::showtime::PlayerConfig,
    motion::walking_engine::Gait,
    sensor::{button::HeadButtons, falling::FallState, imu::IMUValues},
    vision::ball_detection::{ball_tracker::BallTracker, classifier::Balls, Hypothesis},
};

use super::{
    behaviors::{
        CatchFall, CatchFallBehaviorPlugin, ObserveBehaviorPlugin, RlStrikerSearchBehaviorPlugin,
        Sitting, SittingBehaviorPlugin, Stand, StandBehaviorPlugin, StandLookAt,
        StandLookAtBehaviorPlugin, Standup, StandupBehaviorPlugin, StartUpBehaviorPlugin,
        WalkBehaviorPlugin, WalkToBehaviorPlugin, WalkToSet, WalkToSetBehaviorPlugin,
    },
    primary_state::PrimaryState,
    roles::{
        Defender, DefenderRolePlugin, Goalkeeper, GoalkeeperRolePlugin, Striker, StrikerRolePlugin,
    },
};

const FORWARD_LEANING_THRESHOLD: f32 = 0.2;
const BACKWARD_LEANING_THRESHOLD: f32 = -0.2;

pub(super) struct BehaviorEnginePlugin;

impl Plugin for BehaviorEnginePlugin {
    fn build(&self, app: &mut App) {
        // StatesPlugin should be added before init_state
        app.init_state::<BehaviorState>()
            .init_state::<RoleState>()
            .add_plugins((
                StandBehaviorPlugin,
                WalkBehaviorPlugin,
                CatchFallBehaviorPlugin,
                ObserveBehaviorPlugin,
                SittingBehaviorPlugin,
                StandLookAtBehaviorPlugin,
                StandupBehaviorPlugin,
                StartUpBehaviorPlugin,
                WalkToBehaviorPlugin,
                WalkToSetBehaviorPlugin,
                DefenderRolePlugin,
                GoalkeeperRolePlugin,
                StrikerRolePlugin,
                RlStrikerSearchBehaviorPlugin,
            ))
            .add_systems(PostUpdate, role_base);
    }
}

pub trait RlBehaviorInput<T> {
    fn to_input(&self) -> T;
}

pub trait RlBehaviorOutput<T> {
    fn from_output(output: T) -> Self;
}

pub fn spawn_rl_behavior<M, I, O>(
    commands: &mut Commands,
    model_executor: &mut ModelExecutor<M>,
    input: I,
) where
    I: RlBehaviorInput<M::Inputs>,
    O: RlBehaviorOutput<M::Outputs> + Resource,
    M: MlModel,
    <M as ml::MlModel>::Outputs: std::marker::Send,
{
    commands
        .infer_model(model_executor)
        .with_input(&input.to_input())
        .create_resource()
        .spawn(|output| Some(O::from_output(output)));
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BehaviorState {
    Walk,
    Stand,
    CatchFall,
    Observe,
    Sitting,
    StandLookAt,
    Standup,
    #[default]
    StartUp,
    WalkTo,
    WalkToSet,
    RlStrikerSearchBehavior,
}

#[must_use]
pub fn in_behavior<T: Behavior>(state: Option<Res<State<BehaviorState>>>) -> bool {
    match state {
        Some(current_behavior) => *current_behavior == T::STATE,
        None => panic!("Failed to get the current behavior state"),
    }
}

pub trait CommandsBehaviorExt {
    fn set_behavior<T: Behavior>(&mut self, behavior: T);

    fn set_role<T: Roles>(&mut self, role: T);

    fn disable_role(&mut self);
}

impl CommandsBehaviorExt for Commands<'_, '_> {
    fn set_behavior<T: Behavior>(&mut self, behavior: T) {
        self.set_state(T::STATE);
        self.insert_resource(behavior);
    }

    fn set_role<T: Roles>(&mut self, role: T) {
        self.set_state(T::STATE);
        self.insert_resource(role);
    }

    fn disable_role(&mut self) {
        self.set_state(RoleState::Disabled);
    }
}

// Link each behavior data struct with an enum variant of the BehaviorState
pub trait Behavior: Resource {
    const STATE: BehaviorState;
}

#[derive(States, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleState {
    #[default]
    Disabled,
    Striker,
    Goalkeeper,
    Defender,
}

impl RoleState {
    /// Get the default role for each robot based on that robots player number
    pub fn by_player_number(commands: &mut Commands, player_number: u8) {
        match player_number {
            1 => commands.set_role(Goalkeeper),
            5 | 4 => commands.set_role(Striker::WalkToBall),
            _ => commands.set_role(Defender),
        }
    }

    pub fn assign_role(commands: &mut Commands, player_number: u8) {
        // TODO: Check if robots have been penalized, or which robot is closed to the ball etc.
        Self::by_player_number(commands, player_number);
    }
}

// Link each behavior data struct with an enum variant of the Role
pub trait Roles: Resource {
    const STATE: RoleState;
}

#[must_use]
pub fn in_role<T: Roles>(state: Option<Res<State<RoleState>>>) -> bool {
    match state {
        Some(current_behavior) => *current_behavior == T::STATE,
        None => panic!("Failed to get the current role state"),
    }
}

fn robot_is_leaning(imu_values: &IMUValues) -> bool {
    imu_values.angles.y > FORWARD_LEANING_THRESHOLD
        || imu_values.angles.y < BACKWARD_LEANING_THRESHOLD
}

#[allow(clippy::too_many_arguments)]
pub fn role_base(
    mut commands: Commands,
    behavior_state: Res<State<BehaviorState>>,
    gait: Res<State<Gait>>,
    head_buttons: Res<HeadButtons>,
    primary_state: Res<PrimaryState>,
    fall_state: Res<FallState>,
    standup_state: Option<Res<Standup>>,
    player_config: Res<PlayerConfig>,
    ball_tracker: Res<BallTracker>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    imu_values: Res<IMUValues>,
) {
    commands.disable_role();
    let behavior = behavior_state.get();

    if behavior == &BehaviorState::StartUp {
        if *primary_state == PrimaryState::Sitting && robot_is_leaning(&imu_values) {
        } else if *gait == Gait::Sitting || head_buttons.all_pressed() {
            commands.set_behavior(Sitting);
        } else {
            commands.set_behavior(Stand);
        }

        return;
    }

    if *primary_state == PrimaryState::Sitting {
        commands.set_behavior(Sitting);
        return;
    }

    if standup_state.is_some_and(|s| !s.completed()) {
        return;
    }

    // next up, damage prevention and standup motion takes precedence
    match fall_state.as_ref() {
        FallState::Lying(_) => {
            commands.set_behavior(Standup::default());
            return;
        }
        FallState::Falling(_) => {
            if !matches!(*primary_state, PrimaryState::Penalized)
                && !matches!(*primary_state, PrimaryState::Initial)
            {
                commands.set_behavior(CatchFall);
                return;
            }
        }
        FallState::None => {}
    }

    if *gait == Gait::Sitting && *primary_state != PrimaryState::Sitting {
        commands.set_behavior(Stand);
        return;
    }

    if let Some(message) = game_controller_message {
        if message.game_phase == GamePhase::PenaltyShoot {
            if message.kicking_team == player_config.team_number {
                commands.set_role(Striker::WalkWithBall);
            } else {
                commands.set_behavior(Stand);
                return;
            }
        }
    }

    let most_confident_ball = ball_tracker.state();

    let ball_or_origin = if let Hypothesis::Stationary(_) = ball_tracker.cutoff() {
        most_confident_ball.0
    } else {
        Point2::default()
    };


    match *primary_state {
        PrimaryState::Sitting => commands.set_behavior(Sitting),
        PrimaryState::Penalized => {
            commands.set_behavior(Stand);
        }
        PrimaryState::Standby | PrimaryState::Finished | PrimaryState::Calibration => {
            commands.set_behavior(Stand);
        }
        PrimaryState::Initial => {
            commands.set_behavior(StandLookAt {
                target: Point2::default(),
            });
        }
        PrimaryState::Ready => commands.set_behavior(WalkToSet),
        PrimaryState::Set => commands.set_behavior(StandLookAt {
            target: ball_or_origin,
        }),
        PrimaryState::Playing { .. } => {
            RoleState::assign_role(&mut commands, player_config.player_number);
        }
    }
}
