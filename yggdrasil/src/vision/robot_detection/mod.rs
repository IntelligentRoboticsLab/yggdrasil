use std::time::{Duration, Instant};

use crate::core::debug::DebugContext;
use crate::prelude::*;
use crate::vision::util::non_max_suppression;
use crate::vision::{
    camera::Image,
    util::bbox::{Bbox, Xyxy},
};
use bevy::core::FrameCount;
use bevy::prelude::*;
use box_coder::BoxCoder;
use fast_image_resize as fr;
use heimdall::{CameraLocation, Top};
use itertools::Itertools;
use miette::IntoDiagnostic;
use ml::prelude::*;
use ndarray::{Array2, Axis};
use serde_with::{serde_as, DurationMilliSeconds};

mod anchor_generator;
mod box_coder;

use anchor_generator::DefaultBoxGenerator;
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

use super::util::bbox::{ConvertBbox, Cxcywh};

#[serde_as]
#[derive(Resource, Debug, Clone, Deserialize, Serialize, Reflect)]
#[serde(deny_unknown_fields)]
pub struct RobotDetectionConfig {
    confidence_threshold: f32,
    nms_threshold: f32,
    top_k_detections: usize,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    detection_lifetime: Duration,
    input_width: u32,
    input_height: u32,
    num_anchor_boxes: usize,
    feature_map_shape: (usize, usize, usize),
}

impl Config for RobotDetectionConfig {
    const PATH: &'static str = "robot_detection.toml";
}

impl RobotDetectionConfig {
    #[must_use]
    pub const fn box_shape(&self) -> (usize, usize) {
        (self.num_anchor_boxes, 4)
    }

    #[must_use]
    pub const fn score_shape(&self) -> (usize, usize) {
        (self.num_anchor_boxes, 2)
    }
}

pub struct RobotDetectionPlugin;

impl Plugin for RobotDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<RobotDetectionConfig>()
            .init_ml_model::<RobotDetectionModel>()
            .add_systems(
                Update,
                detect_robots.run_if(
                    task_finished::<Image<Top>>.and_then(task_finished::<RobotDetectionData>),
                ),
            )
            .add_systems(
                PostUpdate,
                visualize_detected_robots.run_if(resource_exists_and_changed::<RobotDetectionData>),
            );
    }
}

/// The robot detection model, based on a VGG-like backbone using SSD detection heads.
pub struct RobotDetectionModel;

impl MlModel for RobotDetectionModel {
    type InputElem = u8;
    type OutputElem = f32;

    type InputShape = (Vec<u8>,);
    type OutputShape = (MlArray<f32>, MlArray<f32>);
    const ONNX_PATH: &'static str = "models/robot_detection.onnx";
}

/// A detected robot, with a bounding box and confidence.
#[derive(Component, Debug, Clone, Reflect)]
pub struct DetectedRobot {
    /// The bounding box of the robot in image coordinates.
    pub bbox: Bbox<Xyxy>,
    /// The confidence of the detection.
    pub confidence: f32,
    /// The time the robot was detected at.
    pub timestamp: Instant,
}

/// A fitted field boundary from a given image
#[derive(Clone, Resource)]
pub struct RobotDetectionData {
    /// The detected robots.
    pub detected: Vec<DetectedRobot>,
    /// The image the robots have been detected in.
    pub image: Image<Top>,
    /// The cycle the detection was completed on.
    pub result_cycle: FrameCount,
}

fn detect_robots(
    mut commands: Commands,
    mut model: ResMut<ModelExecutor<RobotDetectionModel>>,
    config: Res<RobotDetectionConfig>,
    image: Res<Image<Top>>,
    cycle: Res<FrameCount>,
) {
    let resized_image = image
        .resized_yuv(
            config.input_width,
            config.input_height,
            fr::ResizeAlg::Nearest,
        )
        .expect("failed to resize image for robot detection");

    commands
        .infer_model(&mut model)
        .with_input(&(resized_image,))
        .create_resource()
        .spawn({
            let config = (*config).clone();
            let cycle = *cycle;
            let image = image.clone();

            move |(box_regression, scores)| {
                let box_regression = box_regression
                    .into_shape(config.box_shape())
                    .into_diagnostic()
                    .expect("received box regression with incorrect shape");
                let scores = scores
                    .into_shape(config.score_shape())
                    .into_diagnostic()
                    .expect("received scores with incorrect shape");

                let detected_robots = postprocess_detections(
                    (image.width(), image.height()),
                    &config,
                    &Vec::new(),
                    box_regression,
                    scores,
                    config.confidence_threshold,
                    config.top_k_detections,
                );

                Some(RobotDetectionData {
                    detected: detected_robots,
                    image: image.clone(),
                    result_cycle: cycle,
                })
            }
        });
}

fn postprocess_detections(
    (image_width, image_height): (usize, usize),
    config: &RobotDetectionConfig,
    known_robots: &[DetectedRobot],
    box_regression: Array2<f32>,
    scores: Array2<f32>,
    threshold: f32,
    k: usize,
) -> Vec<DetectedRobot> {
    let anchor_generator = DefaultBoxGenerator::new(vec![vec![0.4, 0.5], vec![0.85]], 0.15, 0.9);
    let box_coder = BoxCoder::new((10.0, 10.0, 5.0, 5.0));

    let decoded_boxes = box_coder.decode_single(
        box_regression,
        anchor_generator.create_boxes(
            (config.input_width as usize, config.input_height as usize),
            config.feature_map_shape,
        ),
    );

    let (scale_width, scale_height) = (
        image_width as f32 / config.input_width as f32,
        image_height as f32 / config.input_height as f32,
    );

    // take old boxes, and filter out ones that are too old
    let persisted = known_robots
        .iter()
        .filter(|r| r.timestamp.elapsed() < config.detection_lifetime)
        .cloned();

    let filtered_boxes = scores
        .axis_iter(Axis(0))
        .enumerate()
        .filter_map(|(i, s)| {
            let scores = ml::util::softmax(&[s[0], s[1]]);
            if scores[1] < threshold {
                return None;
            }

            let bbox = decoded_boxes.row(i);
            let bbox = Bbox::xyxy(bbox[0], bbox[1], bbox[2], bbox[3]);

            // clamp bbox to image size
            let bbox = bbox.clamp(config.input_width as f32, config.input_height as f32);

            // rescale bboxes to image size
            let bbox = bbox.scaled(scale_width, scale_height);

            Some((bbox, scores[1]))
        })
        .map(|(bbox, confidence)| DetectedRobot {
            bbox,
            confidence,
            timestamp: Instant::now(),
        })
        .chain(persisted)
        .sorted_by(|a, b| b.confidence.total_cmp(&a.confidence))
        .take(k)
        .collect::<Vec<_>>();

    non_max_suppression(
        &filtered_boxes
            .iter()
            .map(|r| (r.bbox, r.confidence))
            .collect_vec(),
        config.nms_threshold,
    )
    .iter()
    .map(|i| filtered_boxes[*i].clone())
    .collect()
}

fn visualize_detected_robots(dbg: DebugContext, robot_data: Res<RobotDetectionData>) {
    let processed_boxes = robot_data.detected.iter().map(
        |DetectedRobot {
             bbox,
             confidence,
             timestamp,
         }| {
            let cxcywh: Bbox<Cxcywh> = bbox.convert();
            let (cx, cy, w, h) = cxcywh.into();

            // rerun expects half width and half height
            let half_w = w / 2.0;
            let half_h = h / 2.0;

            (
                ((cx, cy), (half_w, half_h)),
                format!(
                    "robot: {confidence:.3} ({}ms)",
                    timestamp.elapsed().as_millis()
                ),
            )
        },
    );

    let ((centers, sizes), scores): ((Vec<_>, Vec<_>), Vec<_>) = processed_boxes.unzip();

    dbg.log(
        Top::make_entity_path("detected_robots"),
        &rerun::Boxes2D::from_centers_and_half_sizes(centers, sizes).with_labels(scores),
    );
}
