use std::{f32, sync::Arc, time::Instant};

use bevy::prelude::*;
use ml::{MlModel, MlModelResourceExt, prelude::ModelExecutor};
use nidhogg::types::{FillExt, HeadJoints};
use rerun::{SerializedComponentBatch, external::arrow};
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
    core::{config::layout::LayoutConfig, debug::DebugContext},
    localization::RobotPose,
    motion::walking_engine::{FootSwitchedEvent, Gait, step::Step, step_context::StepContext},
    nao::{Cycle, NaoManager, Priority},
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

        let input = vec![
            normalized_position_x,
            normalized_position_y,
            cos_yaw,
            sin_yaw,
        ];

        eprintln!("input: {input:?}");

        input
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
    last_action: Option<Res<Output>>,
    cycle: Res<Cycle>,
    dbg: DebugContext,
) {
    let input = Input {
        robot_pose: &robot_pose,
        field_width: layout_config.field.width,
        field_height: layout_config.field.length,
        border_strip_width: layout_config.field.border_strip_width,
    };

    let search_obs =
        serialized_component_batch_f32("yggdrasil.components.RlSearchObs", input.to_input());

    dbg.log_with_cycle("rl_search_obs", *cycle, &[search_obs]);

    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

#[must_use]
fn serialized_component_batch_f32<I: IntoIterator<Item = f32>>(
    descriptor: &str,
    iter: I,
) -> SerializedComponentBatch {
    rerun::SerializedComponentBatch::new(
        Arc::new(arrow::array::Float32Array::from_iter_values(iter)),
        rerun::ComponentDescriptor::new(descriptor),
    )
}

fn handle_inference_output(
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    behavior_config: Res<BehaviorConfig>,
    mut nao_manager: ResMut<NaoManager>,
    observe_starting_time: Res<ObserveStartingTime>,
) {
    eprintln!("output: {:?}", output.step);
    eprintln!(
        "after rescale: {:?}\n\n",
        output.step * behavior_config.rl_striker_search.policy_output_scaling
    );
    step_context
        .request_walk(output.step * behavior_config.rl_striker_search.policy_output_scaling);

    let observe_config = &behavior_config.rl_striker_search;
    look_around(
        &mut nao_manager,
        **observe_starting_time,
        observe_config.head_rotation_speed,
        observe_config.head_yaw_max,
        observe_config.head_pitch_max,
    );
}

fn look_around(
    nao_manager: &mut NaoManager,
    starting_time: Instant,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = starting_time.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };

    nao_manager.set_head(
        position,
        HeadJoints::fill(NaoManager::HEAD_STIFFNESS),
        Priority::default(),
    );
}
