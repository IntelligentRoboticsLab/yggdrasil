use bevy::prelude::*;
use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior2::{
        behaviors::{Observe, WalkTo},
        engine::{BehaviorState, CommandsBehaviorExt, Role, Roles},
    },
    core::config::layout::LayoutConfig,
    impl_roles,
    motion::step_planner::{StepPlanner, Target},
};

/// Plugin for the Goalkeeper role
pub struct GoalkeeperRolePlugin;

impl Plugin for GoalkeeperRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, goalkeeper_role.run_if(in_state(Role::Goalkeeper)));
    }
}

/// The [`Goalkeeper`] role is held by a single robot at a time, usually player number 1.
/// It's job is to prevent the ball from entering the goal, which it does by staying in the goal area.
#[derive(Resource)]
pub struct Goalkeeper;
impl_roles!(Goalkeeper, Goalkeeper);

pub fn goalkeeper_role(
    mut commands: Commands,
    step_planner: Res<StepPlanner>,
    layout_config: Res<LayoutConfig>,
    behavior: Res<State<BehaviorState>>,
) {
    let field_length = layout_config.field.length;
    let keeper_target = Target {
        position: Point2::new(-field_length / 2., 0.),
        rotation: Some(UnitComplex::<f32>::from_angle(0.0)),
    };

    if !step_planner.has_target() {
        commands.set_behavior(WalkTo {
            target: keeper_target,
        });
        return;
    }

    if step_planner.reached_target() {
        if behavior.get() != &BehaviorState::Observe {
            commands.set_behavior(Observe::default());
        }
    } else {
        commands.set_behavior(WalkTo {
            target: keeper_target,
        });
    }
}
