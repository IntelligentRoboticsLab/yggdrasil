use bevy::prelude::*;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{CatchFall, Observe, Sitting, Stand, Standup, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, Role, Roles},
        primary_state::PrimaryState,
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walk::engine::WalkingEngine,
    },
    sensor::{button::HeadButtons, falling::FallState},
};

/// Plugin for the Defender role
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, defender_role.run_if(in_role::<Defender>));
    }
}

/// The [`Defender`] role is held by any robot that does not see the ball.
/// It's job is to observe it's set position depending on player number.
#[derive(Resource)]
pub struct Defender;
impl Roles for Defender {
    const STATE: Role = Role::Defender;
}

#[allow(clippy::too_many_arguments)]
pub fn defender_role(
    mut commands: Commands,
    pose: Res<RobotPose>,
    player_config: Res<PlayerConfig>,
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

    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);
    let set_position = set_robot_position.isometry.translation.vector;
    let set_point = Point2::new(set_position.x, set_position.y);
    let defend_target = Target {
        position: set_point,
        rotation: Some(set_robot_position.isometry.rotation),
    };

    let close_to_target = pose.distance_to(&set_point) < 0.4;
    let aligned_with_rotation =
        (pose.world_rotation() - set_robot_position.isometry.rotation.angle()).abs() < 0.2;

    if step_planner.has_target() && step_planner.reached_target()
        || (close_to_target && aligned_with_rotation)
    {
        if behavior.get() != &BehaviorState::Observe {
            commands.set_behavior(Observe::with_turning(-0.4));
        }
    } else {
        commands.set_behavior(WalkTo {
            target: defend_target,
        });
    }
}
