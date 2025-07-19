use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::{
        step_planner::StepPlanner,
        walking_engine::{step::Step, step_context::StepContext},
    },
    nao::{HeadMotionManager, LookAt},
};

use nalgebra::Point3;

pub struct WalkBehaviorPlugin;

impl Plugin for WalkBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk.run_if(in_behavior::<Walk>));
    }
}

/// Perform a specific walk step, whilst looking at a target point.
#[derive(Resource)]
pub struct Walk {
    pub step: Step,
    pub look_target: Option<Point3<f32>>,
}

impl Behavior for Walk {
    const STATE: BehaviorState = BehaviorState::Walk;
}

fn walk(
    walk: Res<Walk>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
    pose: Res<RobotPose>,
) {
    if let Some(point) = walk.look_target {
        head_motion_manager.request_look_at(LookAt {
            pose: *pose,
            point: point,
        });
    }

    step_planner.clear_target();
    step_context.request_walk(walk.step);
}
