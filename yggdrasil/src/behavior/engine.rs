use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase};
use heimdall::{Bottom, Top};
use nalgebra::Point2;

use crate::{
    core::config::showtime::PlayerConfig,
    motion::walking_engine::Gait,
    sensor::{
        button::HeadButtons, falling::FallState, imu::IMUValues, orientation::RobotOrientation,
    },
    vision::ball_detection::classifier::Balls,
};

use super::{
    behaviors::{
        CatchFall, CatchFallBehaviorPlugin, InitialStandLook, InitialStandLookBehaviorPlugin,
        ObserveBehaviorPlugin, Sitting, SittingBehaviorPlugin, Stand, StandBehaviorPlugin,
        StandLookAt, StandLookAtBehaviorPlugin, Standup, StandupBehaviorPlugin,
        StartUpBehaviorPlugin, WalkBehaviorPlugin, WalkToBehaviorPlugin, WalkToSet,
        WalkToSetBehaviorPlugin,
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
            .init_state::<Role>()
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
                InitialStandLookBehaviorPlugin,
            ))
            .add_systems(PostUpdate, role_base);
    }
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
    InitialStandLook,
}

#[must_use]
pub fn in_behavior<T: Behavior>(state: Option<Res<State<BehaviorState>>>) -> bool {
    match state {
        Some(current_behavior) => *current_behavior == T::STATE,
        None => panic!("Failed to get the current behavior state"),
    }
}

fn insert_behavior_resource_if_not_active<B: Behavior>(behavior_resource: B) -> impl Command {
    move |world: &mut World| {
        let behavior_state = world.get_resource::<State<BehaviorState>>();
        if behavior_state.is_none_or(|behavior_state| *behavior_state.get() != B::STATE) {
            world.get_resource_or_insert_with(|| behavior_resource);
        }
    }
}

fn insert_role_resource_if_not_active<R: Roles>(role_resource: R) -> impl Command {
    move |world: &mut World| {
        let role_state = world.get_resource::<State<Role>>();
        if role_state.is_none_or(|role_state| *role_state.get() != R::STATE) {
            world.get_resource_or_insert_with(|| role_resource);
        }
    }
}

pub trait CommandsBehaviorExt {
    fn set_behavior<T: Behavior>(&mut self, behavior: T);

    fn reset_behavior<T: Behavior>(&mut self, behavior: T);

    fn set_role<T: Roles>(&mut self, role: T);

    fn reset_role<T: Roles>(&mut self, role: T);

    fn disable_role(&mut self);
}

impl CommandsBehaviorExt for Commands<'_, '_> {
    fn set_behavior<T: Behavior>(&mut self, behavior: T) {
        self.set_state(T::STATE);
        self.queue(insert_behavior_resource_if_not_active(behavior));
    }

    fn reset_behavior<T: Behavior>(&mut self, behavior: T) {
        self.set_state(T::STATE);
        self.insert_resource(behavior);
    }

    fn set_role<T: Roles>(&mut self, role: T) {
        self.set_state(T::STATE);
        self.queue(insert_role_resource_if_not_active(role));
    }

    fn reset_role<T: Roles>(&mut self, role: T) {
        self.set_state(T::STATE);
        self.insert_resource(role);
    }

    fn disable_role(&mut self) {
        self.set_state(Role::Disabled);
    }
}

// Link each behavior data struct with an enum variant of the BehaviorState
pub trait Behavior: Resource {
    const STATE: BehaviorState;
}

#[derive(States, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    #[default]
    Disabled,
    Striker,
    Goalkeeper,
    Defender,
}

impl Role {
    /// Get the default role for each robot based on that robots player number
    pub fn by_player_number(mut commands: Commands, player_number: u8) {
        // TODO: get the default role for each robot by player number
        match player_number {
            1 => commands.set_role(Goalkeeper),
            5 => commands.set_role(Striker::WalkToBall),
            _ => commands.set_role(Defender),
        }
    }

    pub fn assign_role(mut commands: Commands, sees_ball: bool, player_number: u8) {
        if sees_ball {
            commands.set_role(Striker::WalkToBall);
        } else {
            Self::by_player_number(commands, player_number);
        }
    }
}

// Link each behavior data struct with an enum variant of the Role
pub trait Roles: Resource {
    const STATE: Role;
}

#[must_use]
pub fn in_role<T: Roles>(state: Option<Res<State<Role>>>) -> bool {
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
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    imu_values: Res<IMUValues>,
    mut orientation: ResMut<RobotOrientation>,
) {
    commands.disable_role();
    let behavior = behavior_state.get();

    if behavior == &BehaviorState::StartUp {
        if (!robot_is_leaning(&imu_values) && *gait == Gait::Sitting) || head_buttons.all_pressed()
        {
            commands.set_behavior(Sitting);
        }
        if *primary_state == PrimaryState::Initial {
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
            if !matches!(*primary_state, PrimaryState::Penalized) {
                commands.set_behavior(CatchFall);
                return;
            }
        }
        FallState::None => {}
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

    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    let ball_or_origin = most_confident_ball.unwrap_or(Point2::origin());

    match *primary_state {
        PrimaryState::Sitting => commands.set_behavior(Sitting),
        PrimaryState::Penalized => {
            orientation.reset();
            commands.set_behavior(Stand);
        }
        PrimaryState::Standby | PrimaryState::Finished | PrimaryState::Calibration => {
            commands.set_behavior(Stand);
        }
        PrimaryState::Initial => {
            orientation.reset();
            commands.set_behavior(StandLookAt {
                target: Point2::default(),
            });
        }
        PrimaryState::Ready => commands.set_behavior(WalkToSet {}),
        PrimaryState::Set => commands.set_behavior(StandLookAt {
            target: ball_or_origin,
        }),
        PrimaryState::Playing { .. } => {
            Role::assign_role(
                commands,
                most_confident_ball.is_some(),
                player_config.player_number,
            );
        }
    }
}
