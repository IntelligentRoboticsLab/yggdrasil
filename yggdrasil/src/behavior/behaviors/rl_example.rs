use bevy::prelude::*;
use ml::{prelude::ModelExecutor, MlModel};
use nalgebra::Point2;

type ModelInput = Vec<f32>;
type ModelOutput = (f32, f32, f32);

use crate::{
    behavior::engine::{
        in_behavior, run_rl_behavior, Behavior, BehaviorState, RlBehaviorInput, RlBehaviorOutput,
    },
    localization::RobotPose,
    motion::walking_engine::{step::Step, step_context::StepContext},
};

pub struct RlBehaviorPlugin;

impl Plugin for RlBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, behave.run_if(in_behavior::<RlExampleBehavior>));
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
    target: &'d Point2<f32>,
}

impl RlBehaviorInput<ModelInput> for Input<'_> {
    fn to_input(&self) -> ModelInput {
        let translation = self.robot_pose.inner.translation;
        let rotation = self.robot_pose.inner.rotation;

        vec![
            translation.x,
            translation.y,
            rotation.angle(),
            self.target.x,
            self.target.y,
        ]
    }
}

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

pub fn behave(
    mut commands: Commands,
    mut model_executor: ResMut<ModelExecutor<RlExampleBehaviorModel>>,
    mut step_context: ResMut<StepContext>,
    robot_pose: Res<RobotPose>,
) {
    let target = Point2::new(0.0, 0.0);
    let input = Input {
        robot_pose: &robot_pose,
        target: &target,
    };

    let output: Output = run_rl_behavior(&mut commands, &mut *model_executor, input);
    step_context.request_walk(output.step);
}
