use bevy::prelude::*;
use heimdall::{CameraLocation, Top};
use miette::IntoDiagnostic;
use ml::{
    prelude::{MlTaskCommandsExt, ModelExecutor},
    util::{argmax, softmax},
    MlArray, MlModel, MlModelResourceExt,
};
use ndarray::{Array2, Axis};

use crate::{core::debug::DebugContext, nao::Cycle, vision::camera::Image};

use super::{
    recognize::{recognising_pose, request_recognition},
    DetectRefereePose, RefereePose,
};

// TODO: Probably in a config file
const INPUT_WIDTH: u32 = 256;
const INPUT_HEIGHT: u32 = 256;
const OUTPUT_SHAPE: (usize, usize) = (17, 3);

pub struct RefereePoseDetectionPlugin;

impl Plugin for RefereePoseDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RefereePoseEstimatorModel>()
            .add_event::<DetectRefereePose>()
            .add_event::<RefereePoseDetected>()
            .add_systems(
                Update,
                (
                    detect_referee_pose
                        .after(request_recognition)
                        .after(recognising_pose),
                    send_referee_pose_output,
                    log_estimated_pose,
                    show_pose,
                )
                    .chain(),
            );
    }
}

pub(super) struct RefereePoseEstimatorModel;

impl MlModel for RefereePoseEstimatorModel {
    type Inputs = Vec<u8>;

    type Outputs = (MlArray<f32>, Vec<f32>);

    const ONNX_PATH: &'static str = "models/pose_estimator.onnx";
}

pub fn detect_referee_pose(
    mut commands: Commands,
    mut detect_pose: EventReader<DetectRefereePose>,
    mut model: ResMut<ModelExecutor<RefereePoseEstimatorModel>>,
    image: Res<Image<Top>>,
) {
    for _ev in detect_pose.read() {
        // Resize yuyv
        let resized_image = image
            .yuyv_image()
            .resize(INPUT_WIDTH, INPUT_HEIGHT)
            .expect("Failed to resize image for robot detection");


        // let mut file = File::create("yuv_image.npy").unwrap();
        // file.write_all(&resized_image).unwrap();

        commands
            .infer_model(&mut model)
            .with_input(&resized_image)
            .create_resource()
            .spawn(|model_output| {
                let (keypoints, class_logits) = model_output;

                let estimated_pose = keypoints
                    .to_shape(OUTPUT_SHAPE)
                    .into_diagnostic()
                    .expect("received pose keypoints with incorrect shape")
                    .to_owned();

                let probs = softmax(&class_logits);
                // println!("Class logits: {:?}", class_logits);
                // println!("Class probs {:?}", probs);

                let pose_idx = argmax(&probs);
                let pose = match pose_idx {
                    0 => RefereePose::Idle,
                    1 => RefereePose::GoalKick,
                    2 => RefereePose::Goal,
                    3 => RefereePose::PushingFreeKick,
                    4 => RefereePose::CornerKick,
                    5 => RefereePose::KickIn,
                    6 => RefereePose::Ready,
                    _ => {
                        eprintln!("unknown referee pose");
                        return None;
                    }
                };

                let output = RefereePoseDetectionOutput {
                    keypoints: estimated_pose,
                    pose,
                };
                Some(output)
            });

        break;
    }

    // A single estimate is enough for multiple estimate pose requests so extra
    // events can all be removed.
    detect_pose.clear();
}

pub fn send_referee_pose_output(
    pose_detection_output: Option<Res<RefereePoseDetectionOutput>>,
    mut pose_detected: EventWriter<RefereePoseDetected>,
) {
    if let Some(pose_detection_output) = pose_detection_output {
        if pose_detection_output.is_added() || pose_detection_output.is_changed() {
            pose_detected.send(RefereePoseDetected {
                keypoints: pose_detection_output.keypoints.clone(),
                pose: pose_detection_output.pose.clone(),
            });
        }
    }
}

pub fn log_estimated_pose(
    mut pose_estimated: EventReader<RefereePoseDetected>,
    dbg: DebugContext,
    image: Res<Image<Top>>,
    cycle: Res<Cycle>,
) {
    for pose in pose_estimated.read() {
        let image_height = image.height();
        let image_width = image.width();

        let keypoints: Vec<(f32, f32)> = pose
            .keypoints
            .axis_iter(Axis(0))
            .map(|v| (v[1] * image_width as f32, v[0] * image_height as f32))
            .collect();

        let point_could = rerun::Points2D::new(keypoints).with_labels(vec!(format!("{:?}", pose.pose)));
        dbg.log_with_cycle(
            Top::make_entity_image_path("referee_pose_keypoints"),
            *cycle,
            &point_could,
        );
    }
}

fn show_pose(mut pose_detected: EventReader<RefereePoseDetected>) {
    for _ev in pose_detected.read() {
        println!("Pose detected: {:?}", _ev.pose);
        break;
    }
    pose_detected.clear();
}

#[derive(Resource)]
pub struct RefereePoseDetectionOutput {
    pub keypoints: Array2<f32>,
    pub pose: RefereePose,
}

#[derive(Event)]
pub struct RefereePoseDetected {
    pub keypoints: Array2<f32>,
    pub pose: RefereePose,
}
