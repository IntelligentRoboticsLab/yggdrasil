use std::{f32, time::Duration};

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use ml::{prelude::ModelExecutor, MlModel, MlModelResourceExt};
use nalgebra::{Point2, Point3};
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

type ModelInput = Vec<f32>;
type ModelOutput = Vec<f32>;

use crate::{
    behavior::{
        engine::{
            in_behavior, spawn_rl_behavior, Behavior, BehaviorState, RlBehaviorInput,
            RlBehaviorOutput,
        },
        BehaviorConfig,
    },
    localization::RobotPose,
    motion::walking_engine::{step::Step, step_context::StepContext},
    nao::{NaoManager, Priority},
    vision::ball_detection::classifier::Balls,
};

pub struct RlStrikerSearchBehaviorPlugin;

impl Plugin for RlStrikerSearchBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RlStrikerSearchBehaviorModel>()
            .add_systems(
                Update,
                run_inference
                    .run_if(in_behavior::<RlStrikerSearchBehavior>.and(task_finished::<Output>)),
            )
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
    const STATE: BehaviorState = BehaviorState::RlExampleBehavior;
}

struct Input<'d> {
    robot_pose: &'d RobotPose,
    ball_position: &'d Point2<f32>,
    goal_position: &'d Point2<f32>,

    field_width: f32,
    field_height: f32,
}

impl RlBehaviorInput<ModelInput> for Input<'_> {
    fn to_input(&self) -> ModelInput {
        let robot_position = self.robot_pose.inner.translation.vector.xy();
        let robot_angle = self.robot_pose.inner.rotation.angle();

        let relative_ball_position = self.ball_position - robot_position;
        let relative_ball_angle = relative_ball_position.y.atan2(relative_ball_position.x);

        let relative_goal_position = self.goal_position - robot_position;
        let relative_goal_angle = relative_goal_position.y.atan2(relative_goal_position.x);

        let last_seen_ball_pos_x = (self.ball_position.x - robot_position.x) / self.field_height;
        let last_seen_ball_pos_y = (self.ball_position.y - robot_position.y) / self.field_height;

        vec![
            (self.ball_position.x - robot_position.x) / self.field_height,
            (self.ball_position.y - robot_position.y) / self.field_width,
            (relative_ball_angle - robot_angle).sin(),
            (relative_ball_angle - robot_angle).sin(),
            (self.goal_position.x - self.ball_position.x) / self.field_height,
            (self.goal_position.y - self.ball_position.y) / self.field_width,
            (relative_goal_angle - robot_angle).sin(),
            (relative_goal_angle - robot_angle).cos(),
            last_seen_ball_pos_x,
            last_seen_ball_pos_y,
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
    balls_top: Res<Balls<Top>>,
    balls_bottom: Res<Balls<Bottom>>,
) {
    let Some(most_confident_ball) = balls_bottom
        .most_confident_ball()
        .map(|b| b.position)
        .or(balls_top.most_confident_ball().map(|b| b.position))
    else {
        return;
    };

    let goal_position = Point2::new(4.5, 0.0);

    let input = Input {
        robot_pose: &robot_pose,
        ball_position: &most_confident_ball,
        goal_position: &goal_position,

        field_width: 6.0,
        field_height: 9.0,
    };

    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

fn handle_inference_output(
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    behavior_config: Res<BehaviorConfig>,
    balls_top: Res<Balls<Top>>,
    balls_bottom: Res<Balls<Bottom>>,
    pose: Res<RobotPose>,
    mut nao_manager: ResMut<NaoManager>,
) {
    let Some(most_confident_ball) = balls_bottom
        .most_confident_ball()
        .map(|b| b.position)
        .or(balls_top.most_confident_ball().map(|b| b.position))
    else {
        return;
    };

    step_context
        .request_walk(output.step * behavior_config.rl_striker_search.policy_output_scaling);

    let point3 = Point3::new(most_confident_ball.x, most_confident_ball.y, 0.0);
    let look_at = pose.get_look_at_absolute(&point3);

    nao_manager.set_head_target(
        look_at,
        Duration::from_millis(500),
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
}
