use std::ops::Deref;
use std::time::{Duration, Instant};

use crate::nao::Cycle;
use crate::prelude::*;
use crate::vision::util::non_max_suppression;
use crate::{
    core::{
        debug::DebugContext,
        ml::{self, MlModel, MlTask, MlTaskResource},
    },
    vision::{
        camera::{Image, TopImage},
        util::bbox::{Bbox, ConvertBbox, Cxcywh, Xyxy},
    },
};
use box_coder::BoxCoder;
use fast_image_resize as fr;
use itertools::Itertools;
use miette::IntoDiagnostic;
use ndarray::{Array2, Axis};
use serde_with::{serde_as, DurationMilliSeconds};

mod anchor_generator;
mod box_coder;

use anchor_generator::DefaultBoxGenerator;
use serde::{Deserialize, Serialize};

#[serde_as]
#[derive(Debug, Deserialize, Serialize, Inspect)]
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

pub struct RobotDetectionModule;

impl Module for RobotDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_ml_task::<RobotDetectionModel>()?
            .init_config::<RobotDetectionConfig>()?
            .add_startup_system(init_robot_detection)?
            .add_system_chain((detect_robots, log_detected_robots)))
    }
}

/// The robot detection model, based on a VGG-like backbone using SSD detection heads.
pub struct RobotDetectionModel;

impl MlModel for RobotDetectionModel {
    type InputType = u8;
    type OutputType = f32;
    const ONNX_PATH: &'static str = "models/robot_detection.onnx";
}

/// A detected robot, with a bounding box and confidence.
#[derive(Debug, Clone)]
pub struct DetectedRobot {
    /// The bounding box of the robot in image coordinates.
    pub bbox: Bbox<Xyxy>,
    /// The confidence of the detection.
    pub confidence: f32,
    /// The time the robot was detected at.
    pub timestamp: Instant,
}

/// A fitted field boundary from a given image
#[derive(Clone)]
pub struct RobotDetectionData {
    /// The detected robots.
    pub detected: Vec<DetectedRobot>,
    /// The image the robots have been detected in.
    pub image: Image,
    /// The cycle the detection was completed on.
    pub result_cycle: Cycle,
}

/// For keeping track of the image that a robot inference is running for.
pub struct RobotDetectionImage(Image);

#[startup_system]
fn init_robot_detection(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let robot_detection_image = RobotDetectionImage(top_image.deref().clone());

    // Initialize the field boundary with a single line at the top of the image
    let detected_robots = RobotDetectionData {
        detected: Vec::new(),
        image: top_image.deref().clone(),
        result_cycle: Cycle::default(),
    };

    storage.add_resource(Resource::new(robot_detection_image))?;
    storage.add_resource(Resource::new(detected_robots))?;

    Ok(())
}

fn poll_model(
    model: &mut MlTask<RobotDetectionModel>,
    config: &RobotDetectionConfig,
    robots: &mut RobotDetectionData,
    robot_detection_image: &RobotDetectionImage,
    current_cycle: &Cycle,
) -> Result<()> {
    // poll for result
    let Some(result) = model.poll_multi::<Vec<f32>>().transpose()? else {
        return Ok(());
    };

    let box_regression =
        Array2::from_shape_vec(config.box_shape(), result[0].clone()).into_diagnostic()?;
    let scores =
        Array2::from_shape_vec(config.score_shape(), result[1].clone()).into_diagnostic()?;

    let detected_robots = postprocess_detections(
        (
            robot_detection_image.0.width(),
            robot_detection_image.0.height(),
        ),
        config,
        &robots.detected,
        box_regression,
        scores,
        config.confidence_threshold,
        config.top_k_detections,
    );

    *robots = RobotDetectionData {
        detected: detected_robots,
        image: robot_detection_image.0.clone(),
        result_cycle: *current_cycle,
    };

    Ok(())
}

#[system]
fn detect_robots(
    model: &mut MlTask<RobotDetectionModel>,
    config: &RobotDetectionConfig,
    robots: &mut RobotDetectionData,
    robot_detection_image: &mut RobotDetectionImage,
    top_image: &TopImage,
    cycle: &Cycle,
) -> Result<()> {
    if model.active() {
        poll_model(model, config, robots, robot_detection_image, cycle)?;
        return Ok(());
    }

    if robot_detection_image.0.is_from_cycle(*cycle) {
        return Ok(());
    }

    // start new inference
    let resized_image = top_image.resized_yuv(
        config.input_width,
        config.input_height,
        fr::ResizeAlg::Nearest,
    )?;

    if let Ok(()) = model.try_start_infer(&resized_image) {
        // We need to keep track of the image we started the inference with
        *robot_detection_image = RobotDetectionImage(top_image.deref().clone());
    };

    Ok(())
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

#[system]
fn log_detected_robots(
    robot_data: &RobotDetectionData,
    ctx: &DebugContext,
    current_cycle: &Cycle,
) -> Result<()> {
    if robot_data.result_cycle != *current_cycle {
        return Ok(());
    }

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

    ctx.log_boxes2d_with_class(
        "/top_camera/image/robots",
        &centers,
        &sizes,
        scores,
        robot_data.image.cycle(),
    )?;

    Ok(())
}
