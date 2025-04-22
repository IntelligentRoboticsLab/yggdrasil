use bevy::prelude::*;
use heimdall::{CameraLocation, Top};
use miette::IntoDiagnostic;
use ml::{
    MlArray, MlModel, MlModelResourceExt,
    prelude::{MlTaskCommandsExt, ModelExecutor},
    util::{argmax, softmax},
};
use ndarray::{Array1, Array2, Axis};

use crate::{
    core::debug::DebugContext,
    nao::Cycle,
    vision::{
        camera::{CameraConfig, Image},
        util::resize_image,
    },
};

use super::{
    RefereePose, RefereePoseConfig,
    recognize::{recognizing_pose, request_recognition},
};

pub struct RefereePoseDetectionPlugin;

impl Plugin for RefereePoseDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<RefereePoseDetectionModel>()
            .init_state::<VisualRefereeDetectionStatus>()
            .add_event::<DetectRefereePose>()
            .add_event::<RefereePoseDetected>()
            .add_systems(
                Update,
                (
                    detect_referee_pose
                        .after(request_recognition)
                        .after(recognizing_pose),
                    send_referee_pose_output,
                    log_estimated_pose,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    activate_detection_status
                        .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
                    deactivate_detection_status
                        .run_if(in_state(VisualRefereeDetectionStatus::Active)),
                ),
            );
    }
}

pub struct RefereePoseDetectionModel;

impl MlModel for RefereePoseDetectionModel {
    type Inputs = Vec<u8>;

    type Outputs = (MlArray<f32>, MlArray<f32>);

    const ONNX_PATH: &'static str = "models/yolo11n-pose.onnx";
}

fn detect_referee_pose(
    mut commands: Commands,
    mut detect_pose: EventReader<DetectRefereePose>,
    mut model: ResMut<ModelExecutor<RefereePoseDetectionModel>>,
    image: Res<Image<Top>>,
    camera_config: Res<CameraConfig>,
    referee_pose_config: Res<RefereePoseConfig>,
    cycle: Res<Cycle>,
) {
    if detect_pose.read().last().is_some() {
        let top_camera = &camera_config.top;
        let image_center = (
            (top_camera.width / 2) as usize,
            (top_camera.height / 2) as usize,
        );

        let detection_config = referee_pose_config.detection.clone();

        // Resize yuyv
        let cropped_image = image.get_yuyv_patch(
            image_center,
            detection_config.crop_width as usize,
            detection_config.crop_height as usize,
        );

        let resized_image = resize_image(
            cropped_image,
            detection_config.crop_width,
            detection_config.crop_height,
            detection_config.input_width,
            detection_config.input_height,
        )
        .expect("Failed to resize image for visual referee!");

        std::fs::write(format!("poses/out_{}.yuv", cycle.0), resized_image.clone())
            .expect("failed to write");

        let keypoints_shape = detection_config.keypoints_shape;
        commands
            .infer_model(&mut model)
            .with_input(&resized_image)
            .create_resource()
            .spawn(move |model_output| {
                let (_, keypoints) = model_output;

                let best_pose = keypoints
                    .to_shape((17, 3))
                    .expect("wrong shape homie")
                    .to_owned();

                // let pose_idx = argmax(&probs);
                let pose = RefereePose::Idle;

                println!("pose: {best_pose:?}");

                let output = RefereePoseDetectionOutput {
                    keypoints: best_pose,
                    pose,
                };
                Some(output)
            });
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
                pose: pose_detection_output.pose,
            });
        }
    }
}

fn log_estimated_pose(
    mut pose_estimated: EventReader<RefereePoseDetected>,
    dbg: DebugContext,
    image: Res<Image<Top>>,
    cycle: Res<Cycle>,
    referee_pose_config: Res<RefereePoseConfig>,
) {
    for pose in pose_estimated.read() {
        let detection_config = &referee_pose_config.detection;
        let keypoints: Vec<(f32, f32)> = pose
            .keypoints
            .axis_iter(Axis(0))
            .map(|v| (((v[0] / 256.0) * 640.0), ((v[1] / 256.0) * 480.0) as f32))
            .collect();

        let point_could =
            rerun::Points2D::new(keypoints).with_labels(vec![format!("{:?}", pose.pose)]);
        dbg.log_with_cycle(
            Top::make_entity_image_path("referee_pose_keypoints"),
            *cycle,
            &point_could,
        );
    }
}

/// An bevy event ([`Event`]) that is a request to start detecting a referee pose
#[derive(Event)]
pub struct DetectRefereePose;

#[derive(Resource)]
pub struct RefereePoseDetectionOutput {
    pub keypoints: Array2<f32>,
    pub pose: RefereePose,
}

/// A bevy event [`Event`] for when a referee pose is detected.
/// It contains:
/// - `keypoints`: 17 keypoints of the pose estimate model
/// - `pose`: The pose that is detected (one of the poses in [`RefereePose`])
#[derive(Event)]
pub struct RefereePoseDetected {
    pub keypoints: Array2<f32>,
    pub pose: RefereePose,
}

/// A bevy state ([`States`]) that keeps track of whether the referee pose detection is
/// active or not
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisualRefereeDetectionStatus {
    #[default]
    Inactive,
    Active,
}

fn activate_detection_status(
    mut detect_pose: EventReader<DetectRefereePose>,
    mut next_detection_status: ResMut<NextState<VisualRefereeDetectionStatus>>,
) {
    if detect_pose.read().last().is_some() {
        next_detection_status.set(VisualRefereeDetectionStatus::Active);
    }
}

fn deactivate_detection_status(
    mut pose_detected: EventReader<RefereePoseDetected>,
    mut next_detection_status: ResMut<NextState<VisualRefereeDetectionStatus>>,
) {
    if pose_detected.read().last().is_some() {
        next_detection_status.set(VisualRefereeDetectionStatus::Inactive);
    }
}
