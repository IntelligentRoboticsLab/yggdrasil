use bevy::prelude::*;
use heimdall::{Bottom, Top};

use crate::{
    core::config::showtime::PlayerConfig,
    motion::walking_engine::Gait,
    sensor::{button::HeadButtons, falling::FallState},
    vision::ball_detection::classifier::Balls,
};

use super::{
    behaviors::{
        CatchFall, CatchFallBehaviorPlugin, ObserveBehaviorPlugin, Sitting, SittingBehaviorPlugin,
        Stand, StandBehaviorPlugin, StandLookAtBehaviorPlugin, Standup, StandupBehaviorPlugin,
        StartUpBehaviorPlugin, WalkBehaviorPlugin, WalkToBehaviorPlugin, WalkToSetBehaviorPlugin,
    },
    primary_state::PrimaryState,
    roles::{
        DefenderRolePlugin, Goalkeeper, GoalkeeperRolePlugin, Instinct, InstinctRolePlugin,
        Striker, StrikerRolePlugin,
    },
};

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
                InstinctRolePlugin,
                DefenderRolePlugin,
                GoalkeeperRolePlugin,
                StrikerRolePlugin,
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

    fn set_role<T: Roles>(&mut self, resource: T);

    fn disable_role(&mut self);
}

impl CommandsBehaviorExt for Commands<'_, '_> {
    fn set_behavior<T: Behavior>(&mut self, resource: T) {
        self.set_state(T::STATE);
        self.insert_resource(resource);
    }

    fn set_role<T: Roles>(&mut self, resource: T) {
        self.set_state(T::STATE);
        self.insert_resource(resource);
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
    Instinct,
    Striker,
    Goalkeeper,
    Defender,
    Disabled,
}

impl Role {
    /// Get the default role for each robot based on that robots player number
    pub fn by_player_number(mut commands: Commands, player_number: u8) {
        // TODO: get the default role for each robot by player number
        match player_number {
            1 => commands.set_role(Goalkeeper),
            5 => commands.set_role(Striker::WalkToBall),
            _ => commands.set_role(Instinct),
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

#[allow(clippy::too_many_arguments)]
pub fn role_base(
    mut commands: Commands,
    state: Res<State<BehaviorState>>,
    role: Res<State<Role>>,
    gait: Res<State<Gait>>,
    head_buttons: Res<HeadButtons>,
    primary_state: Res<PrimaryState>,
    fall_state: Res<FallState>,
    standup_state: Option<Res<Standup>>,
    player_config: Res<PlayerConfig>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let behavior = state.get();

    if behavior == &BehaviorState::StartUp {
        if *gait == Gait::Sitting || head_buttons.all_pressed() {
            commands.set_behavior(Sitting);
            commands.disable_role();
        }
        if *primary_state == PrimaryState::Initial {
            commands.set_behavior(Stand);
            commands.disable_role();
        }
        return;
    }

    if *primary_state == PrimaryState::Sitting {
        commands.set_behavior(Sitting);
        commands.disable_role();
        return;
    }

    if standup_state.is_some_and(|s| !s.completed()) {
        return;
    }

    // next up, damage prevention and standup motion takes precedence
    match fall_state.as_ref() {
        FallState::Lying(_) => {
            commands.set_behavior(Standup::default());
            commands.disable_role();
            return;
        }
        FallState::Falling(_) => {
            if !matches!(*primary_state, PrimaryState::Penalized) {
                commands.set_behavior(CatchFall);
                commands.disable_role();
                return;
            }
        }
        FallState::None => {}
    }

    if let PrimaryState::Penalized = primary_state.as_ref() {
        commands.set_behavior(Stand);
        commands.disable_role();
        return;
    }

    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if let Role::Disabled = role.as_ref().get() {
        Role::assign_role(
            commands,
            most_confident_ball.is_some(),
            player_config.player_number,
        );
    }
}
