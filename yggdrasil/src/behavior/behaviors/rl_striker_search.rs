use std::{f32, time::Instant};

use bevy::prelude::*;
use ml::{MlModel, MlModelResourceExt, prelude::ModelExecutor};
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

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
    core::{
        config::layout::LayoutConfig,
        debug::{DebugContext, SerializeComponentBatch},
    },
    localization::RobotPose,
    motion::walking_engine::{FootSwitchedEvent, Gait, step::Step, step_context::StepContext},
    nao::{Cycle, HeadMotionManager},
};

pub struct RlStrikerSearchBehaviorPlugin;

impl Plugin for RlStrikerSearchBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RlStrikerSearchBehaviorModel>()
            .add_systems(
                PreUpdate,
                run_inference
                    .run_if(in_behavior::<RlStrikerSearchBehavior>.and(task_finished::<Output>))
                    .run_if(on_event::<FootSwitchedEvent>.or(in_state(Gait::Standing))),
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

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RlStrikerSearchBehaviorConfig {
    // The output of the policy is element wise multiplied with this value to determine the
    // step that is requested to the walking engine.
    policy_output_scaling: Step,

    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,

    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,

    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
}

pub(super) struct RlStrikerSearchBehaviorModel;

impl MlModel for RlStrikerSearchBehaviorModel {
    type Inputs = ModelInput;
    type Outputs = ModelOutput;

    const ONNX_PATH: &'static str = "models/rl_striker_search_behavior.onnx";
}

#[derive(Resource)]
pub struct RlStrikerSearchBehavior;

impl Behavior for RlStrikerSearchBehavior {
    const STATE: BehaviorState = BehaviorState::RlStrikerSearchBehavior;
}

#[derive(Resource, Deref)]
struct ObserveStartingTime(Instant);

fn reset_observe_starting_time(mut observe_starting_time: ResMut<ObserveStartingTime>) {
    observe_starting_time.0 = Instant::now();
}

struct Input<'d> {
    robot_pose: &'d RobotPose,
    field_width: f32,
    field_height: f32,
    border_strip_width: f32,
}

impl RlBehaviorInput<ModelInput> for Input<'_> {
    fn to_input(&self) -> ModelInput {
        let robot_position = self.robot_pose.inner.translation.vector.xy();
        let robot_angle = self.robot_pose.inner.rotation.angle();

        let normalized_position_x =
            robot_position.x / (self.field_width * 0.5 + self.border_strip_width);
        let normalized_position_y =
            robot_position.y / (self.field_height * 0.5 + self.border_strip_width);

        let cos_yaw = robot_angle.cos();
        let sin_yaw = robot_angle.sin();

        vec![
            normalized_position_x,
            normalized_position_y,
            cos_yaw,
            sin_yaw,
        ]
    }
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

fn run_inference(
    mut commands: Commands,
    mut model_executor: ResMut<ModelExecutor<RlStrikerSearchBehaviorModel>>,
    robot_pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    cycle: Res<Cycle>,
    dbg: DebugContext,
) {
    let input = Input {
        robot_pose: &robot_pose,
        field_width: layout_config.field.width,
        field_height: layout_config.field.length,
        border_strip_width: layout_config.field.border_strip_width,
    };

    dbg.log_with_cycle(
        "behavior/striker_search/observation",
        *cycle,
        &f32::serialize_component_batch("yggdrasil.components.RlSearchObs", input.to_input()),
    );

    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

fn handle_inference_output(
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    behavior_config: Res<BehaviorConfig>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    step_context
        .request_walk(output.step * behavior_config.rl_striker_search.policy_output_scaling);

    head_motion_manager.request_look_around();
}
