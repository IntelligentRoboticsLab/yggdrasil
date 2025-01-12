use bevy::prelude::*;
use heimdall::{Bottom, Top};

use crate::{core::config::showtime::PlayerConfig, vision::ball_detection::classifier::Balls};

use super::{
    behaviors::{
        CatchFallBehaviorPlugin, ObserveBehaviorPlugin, SittingBehaviorPlugin, StandBehaviorPlugin,
        StandLookAtBehaviorPlugin, StandupBehaviorPlugin, StartUpBehaviorPlugin,
        WalkBehaviorPlugin, WalkToBehaviorPlugin, WalkToSetBehaviorPlugin,
    },
    primary_state::{update_primary_state, PrimaryState},
    roles::{DefenderRolePlugin, GoalkeeperRolePlugin, InstinctRolePlugin, StrikerRolePlugin},
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
            .add_systems(Update, behavior.after(update_primary_state));
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BehaviorState {
    Walk,
    #[default]
    Stand,
    CatchFall,
    Observe,
    Sitting,
    StandLookAt,
    Standup,
    StartUp,
    WalkTo,
    WalkToSet,
}

pub trait CommandsBehaviorExt {
    fn set_behavior<T: Behavior>(&mut self, behavior: T);

    fn set_role<T: Roles>(&mut self, resource: T);
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
}

impl Role {
    /// Get the default role for each robot based on that robots player number
    #[must_use]
    pub fn by_player_number(player_number: u8) -> Self {
        // TODO: get the default role for each robot by player number
        match player_number {
            1 => Self::Goalkeeper,
            5 => Self::Striker,
            _ => Self::Defender,
        }
    }

    fn assign_role(sees_ball: bool, player_number: u8) -> Self {
        if sees_ball {
            return Self::Striker;
        }

        Self::by_player_number(player_number)
    }
}

// Link each behavior data struct with an enum variant of the Role
pub trait Roles: Resource {
    const STATE: Role;
}

fn behavior(
    mut commands: Commands,
    player_config: Res<PlayerConfig>,
    primary_state: Res<PrimaryState>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    match *primary_state {
        PrimaryState::Playing { .. } => {
            // Change this to a system, also in Stiker
            let most_confident_ball = bottom_balls
                .most_confident_ball()
                .map(|b| b.position)
                .or(top_balls.most_confident_ball().map(|b| b.position));

            // Only here should we activate the role deciding behavior
            commands.set_state(Role::assign_role(
                most_confident_ball.is_some(),
                player_config.player_number,
            ));
        }
        _ => commands.set_state(Role::Instinct),
    };
}
