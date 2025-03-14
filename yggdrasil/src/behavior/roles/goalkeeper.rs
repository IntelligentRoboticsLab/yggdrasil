use bevy::prelude::*;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::path::{
        geometry::{Isometry, Vector},
        PathPlanner,
    },
};

/// Plugin for the Goalkeeper role
pub struct GoalkeeperRolePlugin;

impl Plugin for GoalkeeperRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, goalkeeper_role.run_if(in_role::<Goalkeeper>));
    }
}

/// The [`Goalkeeper`] role is held by a single robot at a time, usually player number 1.
/// It's job is to prevent the ball from entering the goal, which it does by staying in the goal area.
#[derive(Resource)]
pub struct Goalkeeper;
impl Roles for Goalkeeper {
    const STATE: RoleState = RoleState::Goalkeeper;
}

pub fn goalkeeper_role(
    mut commands: Commands,
    planner: Res<PathPlanner>,
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    behavior: Res<State<BehaviorState>>,
) {
    let field_length = layout_config.field.length;
    let keeper_target = Isometry::new(Vector::new(-field_length / 2., 0.), 0.);

    if planner.reached_target(pose.inner) {
        if behavior.get() != &BehaviorState::Observe {
            commands.set_behavior(Observe::default());
        }
    } else {
        commands.set_behavior(WalkTo {
            target: keeper_target.into(),
        });
    }
}
