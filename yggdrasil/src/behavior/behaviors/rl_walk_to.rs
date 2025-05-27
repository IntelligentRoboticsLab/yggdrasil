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

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RlWalkToBehaviorConfig {
    // The output of the policy is element wise multiplied with this value to determine the
    // step that is requested to the walking engine.
    policy_output_scaling: Step,
}

pub struct RlWalkToBehaviorPlugin;

impl Plugin for RlWalkToBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RlWalkToBehaviorModel>()
            .add_systems(
                Update,
                run_inference
                    .run_if(in_behavior::<RlWalkToBehavior>.and(task_finished::<Output>)),
            )
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

impl RlBehaviorInput<ModelInput> for Input<'_> {
    fn to_input(&self) -> ModelInput {
        // pos = self.robot.get_pos()  # [B, 1, 2]
        // yaw = self.robot.get_yaw()  # [B, 1, 1]
        let robot_position = self.robot_pose.inner.translation.vector.xy();
        let robot_angle = self.robot_pose.inner.rotation.angle();

        // # Relative position to current episode’s target
        // rel_pos = self.target_pos - pos
        // rel_pos /= torch.tensor([self.field.full_field_length, self.field.full_field_width], device=self.device).view(
        //     1, 1, 2
        // )
        let target_position = self.target_position.coords;
        let mut relative_position = target_position - robot_position;
        relative_position.x /= self.field_height;
        relative_position.y /= self.field_width;

        // # Use trigonometric representation for yaw
        // cos_yaw = torch.cos(yaw)  # [B, 1, 1]
        // sin_yaw = torch.sin(yaw)  # [B, 1, 1]
        let cos_angle = robot_angle.cos();
        let sin_angle = robot_angle.sin();

        // # concatenate along the last dimension -> shape [B, 1, 4]
        // obs_batch_dim_preserved = torch.cat([rel_pos, cos_yaw.unsqueeze(-1), sin_yaw.unsqueeze(-1)], dim=-1)
        // obs = obs_batch_dim_preserved.squeeze(1)  # [B, 4]
        // return obs
        vec![
            relative_position.x,
            relative_position.y,
            // note that the angles are not relative
            sin_angle,
            cos_angle,
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

// TODO -> termination condition
//Success ⇨  distance < goal_distance_threshold  AND
//                alignment > goal_alignment_threshold
fn run_inference(
    mut commands: Commands,
    mut model_executor: ResMut<ModelExecutor<RlWalkToBehaviorModel>>,
    robot_pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,   
) {
    // one of the goals
    let target_position = Point2::new(layout_config.field.length, 0.0);

    let input = Input {
        robot_pose: &robot_pose,
        target_position: &target_position,

        field_width: layout_config.field.width,
        field_height: layout_config.field.length,
    };

    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

fn handle_inference_output(
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    behavior_config: Res<BehaviorConfig>,
) {
    step_context
        .request_walk(output.step * behavior_config.rl_walk_to.policy_output_scaling);
}