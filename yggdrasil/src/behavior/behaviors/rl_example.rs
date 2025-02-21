use std::f32;

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use ml::{prelude::ModelExecutor, MlModel};
use nalgebra::Point2;

type ModelInput = Vec<f32>;
type ModelOutput = (f32, f32, f32);

use crate::{
    behavior::engine::{
        in_behavior, spawn_rl_behavior, Behavior, BehaviorState, RlBehaviorInput, RlBehaviorOutput,
    },
    localization::RobotPose,
    motion::walking_engine::{config::WalkingEngineConfig, step::Step, step_context::StepContext},
    vision::ball_detection::classifier::Balls,
};

pub struct RlBehaviorPlugin;

impl Plugin for RlBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            run_inference.run_if(in_behavior::<RlExampleBehavior>),
        )
        .add_systems(
            Update,
            handle_inference_output
                .run_if(in_behavior::<RlExampleBehavior>)
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
            self.ball_position.x / self.field_height,
            self.ball_position.y / self.field_width,
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
        let (forward, left, turn) = output;

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
    mut model_executor: ResMut<ModelExecutor<RlExampleBehaviorModel>>,
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
    config: Res<WalkingEngineConfig>,
) {
    step_context.request_walk(output.step * config.max_step_size);
}
