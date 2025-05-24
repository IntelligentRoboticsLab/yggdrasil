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
                OnEnter(BehaviorState::RlWalkToBehavior),
                reset_observe_starting_time,
            )
            .insert_resource(ObserveStartingTime(Instant::now()))
            .add_systems(
                Update,
                handle_inference_output
                    .after(run_inference)
                    .run_if(in_behavior::<RlWalkToBehavior>)
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
pub struct RlWalkToBehavior;

impl Behavior for RlWalkToBehavior {
    const STATE: BehaviorState = BehaviorState::RlWalkToBehavior;
}

struct Input<'d> {
    robot_pose: &'d RobotPose,
    target_position: &'d Point2<f32>,
    field_width: f32,
    field_height: f32,
}

#[derive(Resource)]
struct Output {
    step: Step,
}

impl RlBehaviorOutput<ModelOutput> for Output {
    fn from_output(output: ModelOutput) -> Self {
        let forward = output[0].clamp(-1.0, 1.0);
        let left = output[1].clamp(-1.0, 1.0);
        let turn = output[2].clamp(-1.0, 1.0);

        Self {
            step: Step {
                forward,
                left,
                turn,
            },
        }
    }
}

// TODO -> termination condition
//Success ⇨  distance < goal_distance_threshold  AND
//                alignment > goal_alignment_threshold
fn walk_to(
    walk_to: Res<RlWalkToBehavior>,
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
}
