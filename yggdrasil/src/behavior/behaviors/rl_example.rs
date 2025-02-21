use std::{f32, ops::DerefMut, time::Duration};

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use ml::{prelude::ModelExecutor, MlModel, MlModelResourceExt};
use nalgebra::{Point2, Point3};

type ModelInput = Vec<f32>;
type ModelOutput = Vec<f32>;

use crate::{
    behavior::engine::{
        in_behavior, spawn_rl_behavior, Behavior, BehaviorState, RlBehaviorInput, RlBehaviorOutput,
    },
    localization::RobotPose,
    motion::walking_engine::{config::WalkingEngineConfig, step::Step, step_context::StepContext},
    nao::{NaoManager, Priority},
    vision::ball_detection::classifier::Balls,
};

pub struct RlBehaviorPlugin;

impl Plugin for RlBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RunningInference(false))
            .init_ml_model::<RlExampleBehaviorModel>()
            .add_systems(
                Update,
                run_inference.run_if(in_behavior::<RlExampleBehavior>),
            )
            .add_systems(
                Update,
                handle_inference_output
                    .after(run_inference)
                    .run_if(in_behavior::<RlExampleBehavior>)
                    .run_if(resource_exists_and_changed::<Output>),
            )
            .add_systems(
                Update,
                reset_inference
                    .after(handle_inference_output)
                    .run_if(resource_exists_and_changed::<Output>),
            );
    }
}

pub(super) struct RlExampleBehaviorModel;

impl MlModel for RlExampleBehaviorModel {
    type Inputs = ModelInput;
    type Outputs = ModelOutput;

    const ONNX_PATH: &'static str = "models/rl_example_behavior.onnx";
}

#[derive(Resource)]
pub struct RlExampleBehavior;

impl Behavior for RlExampleBehavior {
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
        let forward = output[0];
        let left = output[1];
        let turn = output[2];

        Self {
            step: Step {
                forward,
                left,
                turn,
            },
        }
    }
}

#[derive(Resource)]
struct RunningInference(pub bool);

fn run_inference(
    mut commands: Commands,
    mut model_executor: ResMut<ModelExecutor<RlExampleBehaviorModel>>,
    robot_pose: Res<RobotPose>,
    balls_top: Res<Balls<Top>>,
    balls_bottom: Res<Balls<Bottom>>,
    mut running_inference: ResMut<RunningInference>,
) {
    let Some(most_confident_ball) = balls_bottom
        .most_confident_ball()
        .map(|b| b.position)
        .or(balls_top.most_confident_ball().map(|b| b.position))
    else {
        return;
    };

    if running_inference.0 {
        return;
    }

    let goal_position = Point2::new(4.5, 0.0);

    let input = Input {
        robot_pose: &robot_pose,
        ball_position: &most_confident_ball,
        goal_position: &goal_position,

        field_width: 6.0,
        field_height: 9.0,
    };

    running_inference.deref_mut().0 = true;
    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

fn reset_inference(mut running_inference: ResMut<RunningInference>) {
    running_inference.deref_mut().0 = false;
}

fn handle_inference_output(
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    config: Res<WalkingEngineConfig>,
    balls_top: Res<Balls<Top>>,
    balls_bottom: Res<Balls<Bottom>>,
    pose: Res<RobotPose>,
    mut nao_manager: ResMut<NaoManager>,
    running_inference: Res<RunningInference>,
) {
    let Some(most_confident_ball) = balls_bottom
        .most_confident_ball()
        .map(|b| b.position)
        .or(balls_top.most_confident_ball().map(|b| b.position))
    else {
        return;
    };

    step_context.request_walk(output.step * config.max_step_size);

    let point3 = Point3::new(most_confident_ball.x, most_confident_ball.y, 0.0);
    let look_at = pose.get_look_at_absolute(&point3);

    nao_manager.set_head_target(
        look_at,
        Duration::from_millis(500),
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
}
