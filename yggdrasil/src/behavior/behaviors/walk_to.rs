use std::time::Duration;

use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{NaoManager, Priority},
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct WalkToBehaviorPlugin;

impl Plugin for WalkToBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to.run_if(in_behavior::<WalkTo>));
    }
}

#[derive(Resource)]
pub struct WalkTo {
    pub target: Target,
}

impl Behavior for WalkTo {
    const STATE: BehaviorState = BehaviorState::WalkTo;
}

pub fn walk_to(
    walk_to: Res<WalkTo>,
    pose: Res<RobotPose>,
    mut planner: ResMut<crate::motion::path::PathPlanner>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
) {
    let target_point = Point3::new(walk_to.target.position.x, walk_to.target.position.y, 0.0);

    let look_at = pose.get_look_at_absolute(&target_point);
    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    // Check and clear existing target if different
    if step_planner
        .current_absolute_target()
        .is_some_and(|target| target != &walk_to.target)
    {
        step_planner.clear_target();
    }

    // Set absolute target if not set
    step_planner.set_absolute_target_if_unset(walk_to.target);

    let position = match walk_to.target.rotation {
        Some(rotation) => {
            nalgebra::Isometry2::<f32>::from_parts(walk_to.target.position.into(), rotation).into()
        }
        None => walk_to.target.position.into(),
    };

    if planner.target().map_or(true, |target| target.distance(position) >= planner.settings().target_tolerance) {
        planner.set_target(Some(position));
    }

    // Plan step or stand
    if let Some(step) = planner.step(pose.inner.into()) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }
}
