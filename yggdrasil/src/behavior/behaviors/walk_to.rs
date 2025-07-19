use std::time::{Duration, Instant};

use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{HeadMotionManager, NaoManager, Priority},
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct WalkToBehaviorPlugin;

impl Plugin for WalkToBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to.run_if(in_behavior::<WalkTo>))
            .add_systems(OnEnter(BehaviorState::WalkTo), reset_observe_starting_time)
            .insert_resource(ObserveStartingTime(Instant::now()));
    }
}

fn reset_observe_starting_time(mut observe_starting_time: ResMut<ObserveStartingTime>) {
    observe_starting_time.0 = Instant::now();
}

#[derive(PartialEq)]
pub enum LookMode {
    AtTarget,
    Observe,
}

#[derive(Resource)]
pub struct WalkTo {
    pub target: Target,
    pub look_mode: LookMode,
}

impl Behavior for WalkTo {
    const STATE: BehaviorState = BehaviorState::WalkTo;
}

#[derive(Resource, Deref)]
struct ObserveStartingTime(Instant);

fn walk_to(
    walk_to: Res<WalkTo>,
    pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    let target_point = Point3::new(walk_to.target.position.x, walk_to.target.position.y, 0.0);

    if walk_to.look_mode == LookMode::AtTarget {
        let look_at = pose.get_look_at_absolute(&target_point);
        nao_manager.set_head_target(
            look_at,
            HEAD_ROTATION_TIME,
            Priority::default(),
            NaoManager::HEAD_STIFFNESS,
        );
    } else if walk_to.look_mode == LookMode::Observe {
        head_motion_manager.request_look_around();
    }

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
