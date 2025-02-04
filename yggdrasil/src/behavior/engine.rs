use bevy::prelude::*;

use super::{
    behaviors::{
        CatchFallBehaviorPlugin, ObserveBehaviorPlugin, SittingBehaviorPlugin, StandBehaviorPlugin,
        StandLookAtBehaviorPlugin, StandupBehaviorPlugin, StartUpBehaviorPlugin,
        WalkBehaviorPlugin, WalkToBehaviorPlugin, WalkToSetBehaviorPlugin,
    },
    roles::{
        Defender, DefenderRolePlugin, Goalkeeper, GoalkeeperRolePlugin, InstinctRolePlugin,
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
            ));
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
