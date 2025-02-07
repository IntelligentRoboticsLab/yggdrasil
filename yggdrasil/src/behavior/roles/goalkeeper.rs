use bevy::prelude::*;
use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{CatchFall, Observe, Sitting, Stand, Standup, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, Role, Roles},
        primary_state::PrimaryState,
    },
    core::config::layout::LayoutConfig,
    motion::{
        step_planner::{StepPlanner, Target},
        walk::engine::WalkingEngine,
    },
    sensor::{button::HeadButtons, falling::FallState},
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
    const STATE: Role = Role::Goalkeeper;
}

#[allow(clippy::too_many_arguments)]
pub fn goalkeeper_role(
    mut commands: Commands,
    layout_config: Res<LayoutConfig>,
    step_planner: ResMut<StepPlanner>,
    behavior: Res<State<BehaviorState>>,
    primary_state: Res<PrimaryState>,
    walking_engine: Res<WalkingEngine>,
    head_buttons: Res<HeadButtons>,
    standup_state: Option<Res<Standup>>,
    fall_state: Res<FallState>,
) {
    if behavior.as_ref() == &BehaviorState::StartUp {
        if walking_engine.is_sitting() || head_buttons.all_pressed() {
            commands.set_behavior(Sitting);
        }
        if *primary_state == PrimaryState::Initial {
            commands.set_behavior(Stand);
        }
        return;
    }

    // unstiff has the number 1 precedence
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
            }
            return;
        }
        FallState::None => {}
    }

    if let PrimaryState::Penalized = primary_state.as_ref() {
        commands.set_behavior(Stand);
        return;
    }

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
