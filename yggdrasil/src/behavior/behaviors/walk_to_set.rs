use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use nalgebra::{Point2, Point3};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState, RoleState},
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{NaoManager, Priority},
};

/// To prevent the Goalkeeper from walking into the goalpost, we use this position for a better approach.
const GOAL_KEEPER_PRE_SET_POS: Target = Target {
    position: Point2::new(-2.85, 0.0),
    rotation: None,
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct WalkToSetBehaviorPlugin;

impl Plugin for WalkToSetBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_set.run_if(in_behavior::<WalkToSet>));
    }
}

/// Walk to the set position of the robot.
/// Only the Goalkeeper will first walk to the pre-set position before walking to the set position.
#[derive(Resource)]
pub struct WalkToSet {
    // pub is_goalkeeper: bool,
}

impl Behavior for WalkToSet {
    const STATE: BehaviorState = BehaviorState::WalkToSet;
}

pub fn walk_to_set(
    pose: Res<RobotPose>,
    mut planner: ResMut<crate::motion::path::PathPlanner>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
    step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    role: Res<State<RoleState>>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let set_position: Point2<f32> = set_robot_position.isometry.translation.vector.into();

    let look_at = pose.get_look_at_absolute(&Point3::new(
        set_position.x,
        set_position.y,
        RobotPose::CAMERA_HEIGHT,
    ));

    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    let reached_pre_set = !step_planner.has_target()
        || (step_planner
            .current_absolute_target()
            .is_some_and(|current_target| current_target == &GOAL_KEEPER_PRE_SET_POS)
            && !step_planner.reached_target());

    let is_goalkeeper = role.get() == &RoleState::Goalkeeper;

    let position = if is_goalkeeper && reached_pre_set {
            GOAL_KEEPER_PRE_SET_POS.position.into()
    } else {
            set_robot_position.isometry.into()
    };

    if planner.target().map_or(true, |target| target.distance(position) >= planner.settings().target_tolerance) {
        planner.set_target(Some(position));
    }

    if let Some(step) = planner.step(pose.inner.into()) {
        step_context.request_walk(step);
    } else {
        let look_at = pose.get_look_at_absolute(&Point3::origin());
        nao_manager.set_head(look_at, HeadJoints::fill(0.5), Priority::default());

        step_context.request_stand();
    }
}
