use std::{f32, time::{Instant, Duration}};

use bevy::prelude::*;
use ml::{MlModel, MlModelResourceExt, prelude::ModelExecutor};
use nalgebra::Point2;
use nidhogg::types::FillExt;
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

// define aliases
type ModelInput = Vec<f32>;
type ModelOutput = Vec<f32>;

use crate::{
    behavior::{
        BehaviorConfig,
        engine::{
            Behavior, BehaviorState, RlBehaviorInput, RlBehaviorOutput, in_behavior,
            spawn_rl_behavior,
        },
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::walking_engine::{step::Step, step_context::StepContext},
    nao::{NaoManager, Priority},
};

// we want the robot to look at the target
const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct RlWalkToBehaviorPlugin;

impl Plugin for RlWalkToBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RlWalkToBehaviorModel>()
            .add_systems(
                Update,
                run_inference
                    .run_if(in_behavior::<RlStrikerSearchBehavior>.and(task_finished::<Output>)),
            )
            .add_systems(
                OnEnter(BehaviorState::RlStrikerSearchBehavior),
                reset_observe_starting_time,
            )
            .insert_resource(ObserveStartingTime(Instant::now()))
            .add_systems(
                Update,
                handle_inference_output
                    .after(run_inference)
                    .run_if(in_behavior::<RlStrikerSearchBehavior>)
                    .run_if(resource_exists_and_changed::<Output>),
            );
    }
}

pub(super) struct RlWalkToBehaviorModel;

impl MlModel for RlWalkToBehaviorModel {
    /// The model input shape.
    type Inputs = ModelInput;

    /// The model output shape.
    type Outputs = ModelOutput;

    /// Path to the model's ONNX file.
    const ONNX_PATH: &'static str = "models/rl_walk_to.onnx";
}

#[derive(Resource)]
pub struct RlWalkToBehavior {
    target: Point2<f32>, // need to verify if this is the correct input for the model
}

impl Behavior for RlWalkToBehavior {
    const STATE: BehaviorState = BehaviorState::RlWalkToBehavior;
}

#[derive(Resource, Deref)]
struct ObserveStartingTime(Instant);

fn reset_observe_starting_time(mut observe_starting_time: ResMut<ObserveStartingTime>) {
    observe_starting_time.0 = Instant::now();
}

struct Input<'d> {
    robot_pose: &'d RobotPose,
    goal_position: &'d Point2<f32>,

    field_width: f32,
    field_height: f32,
}

fn walk_to(
    walk_to: Res<RlWalkToBehavior>,
    pose: Res<RobotPose>,
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

    // Plan step or stand
    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }
}
