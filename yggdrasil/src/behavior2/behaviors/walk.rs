use bevy::prelude::*;
// use bevy::state::state_scoped::StateScoped;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior2::engine::{Behavior, BehaviorState},
    impl_behavior,
    localization::RobotPose,
    motion::{
        step_planner::StepPlanner,
        walk::engine::{Step, WalkingEngine},
    },
    nao::{NaoManager, Priority},
};

use nalgebra::Point3;

pub struct WalkBehaviorPlugin;

impl Plugin for WalkBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            walk.run_if(in_state(BehaviorState::Walk))
                .run_if(resource_exists::<Walk>),
        );
    }
}

/// Perform a specific walk step, whilst looking at a target point.
#[derive(Resource)]
pub struct Walk {
    pub step: Step,
    pub look_target: Option<Point3<f32>>,
}

impl_behavior!(Walk, Walk);

pub fn walk(
    walk: Res<Walk>,
    mut step_planner: ResMut<StepPlanner>,
    mut walking_engine: ResMut<WalkingEngine>,
    mut nao_manager: ResMut<NaoManager>,
    pose: Res<RobotPose>,
) {
    if let Some(point) = walk.look_target {
        let look_at = pose.get_look_at_absolute(&point);
        nao_manager.set_head(look_at, HeadJoints::fill(0.5), Priority::High);
    }

    step_planner.clear_target();
    walking_engine.request_walk(walk.step);
}
