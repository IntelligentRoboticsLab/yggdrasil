use std::time::Duration;

use bevy::prelude::*;
use nalgebra as na;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    localization::RobotPose,
    motion::{
        path::{geometry::Isometry, PathPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{NaoManager, Priority},
};

const TARGET_OFFSET: na::Point2<f32> = na::point![0., 1.];
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
    mut planner: ResMut<PathPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
) {
    let isometry = Isometry::new(
        walk_to.target.to_point().coords,
        walk_to.target.angle().unwrap_or(pose.inner.rotation.angle()),
    );

    let point = isometry * TARGET_OFFSET;
    let look_at = pose.get_look_at_absolute(&na::point![point.x, point.y, 0.]);

    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    planner.target = Some(walk_to.target);

    match planner.step(pose.inner) {
        Some(step) => step_context.request_walk(step),
        None => step_context.request_stand(),
    }
}
