use bevy::prelude::*;
use ml::{prelude::*, MlModel, MlModelResourceExt};

use super::{
    estimator::{send_referee_pose_estimate_output, RefereePoseEstimated},
    RefereePose,
};

pub struct RefereePoseClassifierPlugin;

impl Plugin for RefereePoseClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RefereePoseClassifierModel>()
            .add_event::<RefereePoseDetected>()
            .add_systems(
                Update,
                (
                    classify_referee_pose.after(send_referee_pose_estimate_output),
                    send_referee_pose_output,
                    show_pose,
                ),
            );
    }
}

pub(super) struct RefereePoseClassifierModel;

impl MlModel for RefereePoseClassifierModel {
    type Inputs = Vec<f32>;

    type Outputs = Vec<f32>;

    const ONNX_PATH: &'static str = "models/simple_pose_classification.onnx";
}

fn classify_referee_pose(
    mut commands: Commands,
    mut pose_estimated: EventReader<RefereePoseEstimated>,
    mut model: ResMut<ModelExecutor<RefereePoseClassifierModel>>,
) {
    for event in pose_estimated.read() {
        // let input = &event.estimated_pose.flatten().to_vec();
        let input: Vec<f32> = vec![0.; 17 * 2];
        println!("Start the pose classify");
        commands
            .infer_model(&mut model)
            .with_input(&input)
            .create_resource()
            .spawn(|_model_output| {
                let pose_idx = 0;
                // TODO: Needs an actualy conversion
                let pose = match pose_idx {
                    0 => RefereePose::FullTime,
                    _ => RefereePose::Idle,
                };
                Some(RefereePoseDetectionOutput { pose })
            });
    }
}

fn send_referee_pose_output(
    pose_detection_output: Option<Res<RefereePoseDetectionOutput>>,
    mut pose_detected: EventWriter<RefereePoseDetected>,
) {
    if let Some(pose_detection_output) = pose_detection_output {
        if pose_detection_output.is_added() || pose_detection_output.is_changed() {
            println!("Done referee pose output");
            pose_detected.send(RefereePoseDetected {
                pose: pose_detection_output.pose.clone(),
            });
        }
    }
}

fn show_pose(mut pose_detected: EventReader<RefereePoseDetected>) {
    for _ev in pose_detected.read() {
        println!("Pose detected: {:?}", _ev.pose);
        break;
    }
    pose_detected.clear();
}

#[derive(Event)]
pub struct RefereePoseDetected {
    pose: RefereePose,
}

#[derive(Resource)]
pub struct RefereePoseDetectionOutput {
    pose: RefereePose,
}
