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
    target: Target,
    look_at: Point3<f32>,
}

impl WalkTo {
    #[must_use]
    pub fn new(target: Target) -> Self {
        Self {
            target,
            look_at: Point3::new(target.position.x, target.position.y, 0.0),
        }
    }

    #[must_use]
    pub fn new_with_look_at(target: Target, look_at: Point3<f32>) -> Self {
        Self { target, look_at }
    }
}

impl Behavior for WalkTo {
    const STATE: BehaviorState = BehaviorState::WalkTo;
}

pub fn walk_to(
    walk_to: Res<WalkTo>,
    pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
) {
    let look_head_joints = pose.get_look_at_absolute(&walk_to.look_at);
    nao_manager.set_head_target(
        look_head_joints,
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

    // Plan step or stand
    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }
}
