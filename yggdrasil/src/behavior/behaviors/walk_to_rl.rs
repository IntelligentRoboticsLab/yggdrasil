use std::time::Instant;

use bevy::prelude::*;
use ml::{MlModel, MlModelResourceExt, prelude::ModelExecutor};

type ModelInput = Vec<f32>;
type ModelOutput = Vec<f32>;
use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};
use tasks::conditions::task_finished;

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
    motion::{
        step_planner::Target,
        walking_engine::{FootSwitchedEvent, Gait, step::Step, step_context::StepContext},
    },
    nao::{NaoManager, Priority},
};

pub struct WalkToRLBehaviorPlugin;

impl Plugin for WalkToRLBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<WalkToRLModel>()
            .add_systems(
                PreUpdate,
                run_inference
                    .run_if(in_behavior::<WalkToRL>.and(task_finished::<Output>))
                    .run_if(on_event::<FootSwitchedEvent>.or(in_state(Gait::Standing))),
            )
            .add_systems(
                OnEnter(BehaviorState::WalkToRL),
                reset_observe_starting_time,
            )
            .insert_resource(ObserveStartingTime(Instant::now()))
            .add_systems(
                Update,
                walk_to_rl
                    .after(run_inference)
                    .run_if(in_behavior::<WalkToRL>)
                    .run_if(resource_exists_and_changed::<Output>),
            );
    }
}

#[derive(Resource)]
pub struct WalkToRL {
    pub target: Target,
}

impl Behavior for WalkToRL {
    const STATE: BehaviorState = BehaviorState::WalkToRL;
}

pub(super) struct WalkToRLModel;

impl MlModel for WalkToRLModel {
    type Inputs = ModelInput;
    type Outputs = ModelOutput;

    const ONNX_PATH: &'static str = "models/walktopoint_sidestep.onnx";
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
    target: Point2<f32>,
}

impl RlBehaviorInput<ModelInput> for Input<'_> {
    fn to_input(&self) -> ModelInput {
        let robot_position = self.robot_pose.inner.translation.vector.xy();
        let robot_angle = self.robot_pose.inner.rotation.angle();

        let relative_x = self.target.x - robot_position.x;
        let relative_y = self.target.y - robot_position.y;

        let normalized_position_x = relative_x / (self.field_width + self.border_strip_width * 2.0);
        let normalized_position_y =
            relative_y / (self.field_height + self.border_strip_width * 2.0);

        // print the normalized position
        // println!(
        //     "Normalized Position: ({}, {})",
        //     normalized_position_x, normalized_position_y
        // );

        // compute the arctan2 of the robot's normalized position
        // to get the angle in radians
        // and then convert it to cosine and sine
        // to get the direction of the robot
        // Compute absolute angle to the target

        let angle_to_target = relative_y.atan2(relative_x); // same as atan2(y, x)

        // Relative angle (target - heading)
        let mut delta_yaw = angle_to_target - robot_angle;
        if delta_yaw > std::f32::consts::PI {
            delta_yaw -= 2.0 * std::f32::consts::PI;
        } else if delta_yaw < -std::f32::consts::PI {
            delta_yaw += 2.0 * std::f32::consts::PI;
        }

        let cos_yaw = delta_yaw.cos();
        let sin_yaw = delta_yaw.sin();

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
    mut model_executor: ResMut<ModelExecutor<WalkToRLModel>>,
    robot_pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    walk_to_rl: Res<WalkToRL>,
) {
    let input = Input {
        robot_pose: &robot_pose,
        field_width: layout_config.field.width,
        field_height: layout_config.field.length,
        border_strip_width: layout_config.field.border_strip_width,
        target: walk_to_rl.target.position,
    };

    spawn_rl_behavior::<_, _, Output>(&mut commands, &mut *model_executor, input);
}

fn walk_to_rl(
    walk_to: Res<WalkToRL>,
    mut step_context: ResMut<StepContext>,
    output: Res<Output>,
    behavior_config: Res<BehaviorConfig>,
    observe_starting_time: Res<ObserveStartingTime>,
    mut nao_manager: ResMut<NaoManager>,
    robot_pose: Res<RobotPose>,
) {
    // let target_point = Point3::new(walk_to.target.position.x, walk_to.target.position.y, 0.0);

    // let look_at = pose.get_look_at_absolute(&target_point);
    // nao_manager.set_head_target(
    //     look_at,
    //     HEAD_ROTATION_TIME,
    //     Priority::default(),
    //     NaoManager::HEAD_STIFFNESS,
    // );

    // println!("Step from model: {:?}", output.step);

    // check if the target position is reached
    // let distance_to_target =
    //     walk_to.target.position.coords - robot_pose.world_position().xy().coords;

    // compute the distance to the target by using the norm
    let difference = walk_to.target.position.coords - robot_pose.world_position().xy().coords;
    let distance_to_target = difference.norm();

    if distance_to_target < 0.1 {
        println!("Target position reached, stopping.");
        step_context.request_stand();
        return;
    }

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
