use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use nalgebra as na;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState, RoleState},
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{path::PathPlanner, walking_engine::step_context::StepContext},
    nao::{NaoManager, Priority},
};

/// To prevent the Goalkeeper from walking into the goalpost, we use this position for a better approach.
const GOAL_KEEPER_PRE_SET_POS: na::Point2<f32> = na::Point2::new(-2.85, 0.0);
const GOAL_KEEPER_PRE_SET_DIST: f32 = 0.05;

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct WalkToSetBehaviorPlugin;

impl Plugin for WalkToSetBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_set.run_if(in_behavior::<WalkToSet>));
    }
}

/// Walk to the set position of the robot.
/// Only the Goalkeeper will first walk to the pre-set position before walking to the set position.
#[derive(Default, Resource)]
pub struct WalkToSet {
    pub reached_pre_set: bool,
}

impl Behavior for WalkToSet {
    const STATE: BehaviorState = BehaviorState::WalkToSet;
}

#[allow(clippy::too_many_arguments)]
pub fn walk_to_set(
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
    mut planner: ResMut<PathPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    mut walk_to_set: ResMut<WalkToSet>,
    role: Res<State<RoleState>>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let set_position: na::Point2<f32> = set_robot_position.isometry.translation.vector.into();

    let look_at = pose.get_look_at_absolute(&na::Point3::new(
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

    if matches!(role.get(), RoleState::Goalkeeper) && !walk_to_set.reached_pre_set {
        let dist = na::distance(&pose.world_position(), &GOAL_KEEPER_PRE_SET_POS);

        if dist <= GOAL_KEEPER_PRE_SET_DIST {
            walk_to_set.reached_pre_set = true;
        } else {
            planner.target = Some(GOAL_KEEPER_PRE_SET_POS.into());
        }
    } else {
        planner.target = Some(set_robot_position.isometry.into());
    }

    if let Some(step) = planner.step(pose.inner) {
        step_context.request_walk(step);
    } else {
        let look_at = pose.get_look_at_absolute(&na::Point3::origin());
        nao_manager.set_head(look_at, HeadJoints::fill(0.5), Priority::default());

        step_context.request_stand();
    }
}
