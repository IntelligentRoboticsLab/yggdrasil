use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    localization::RobotPose,
    motion::{
        step_planner::StepPlanner,
        walkv4::{step::Step, step_context::StepContext},
    },
    nao::{NaoManager, Priority},
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

pub fn walk(
    walk: Res<Walk>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    pose: Res<RobotPose>,
) {
    if let Some(point) = walk.look_target {
        let look_at = pose.get_look_at_absolute(&point);
        nao_manager.set_head_target(look_at, Duration::from_millis(500), Priority::High, 0.5);
    }

    step_planner.clear_target();
    step_context.request_walk(walk.step);
}
