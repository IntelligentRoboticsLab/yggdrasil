use std::{f32, time::Instant};

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use ml::{prelude::ModelExecutor, MlModel, MlModelResourceExt};
use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};
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

        // vec![
        //     (self.ball_position.x - robot_position.x) / self.field_height,
        //     (self.ball_position.y - robot_position.y) / self.field_width,
        //     (relative_ball_angle - robot_angle).sin(),
        //     (relative_ball_angle - robot_angle).sin(),
        //     (self.goal_position.x - self.ball_position.x) / self.field_height,
        //     (self.goal_position.y - self.ball_position.y) / self.field_width,
        //     (relative_goal_angle - robot_angle).sin(),
        //     (relative_goal_angle - robot_angle).cos(),
        //     last_seen_ball_pos_x,
        //     last_seen_ball_pos_y,
        // ]
        vec![
            0.0,
            0.0,
            0.0,
            0.0,
            (self.goal_position.x - robot_position.x) / self.field_height,
            (self.goal_position.y - robot_position.y) / self.field_width,
            (relative_goal_angle - robot_angle).sin(),
            (relative_goal_angle - robot_angle).cos(),
            0.0,
            0.0,
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
    observe_starting_time: Res<ObserveStartingTime>,
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

    let observe_config = &behavior_config.observe;
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
