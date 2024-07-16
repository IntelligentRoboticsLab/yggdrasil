use std::ops::Deref;

use crate::nao::Cycle;
use crate::prelude::*;
use crate::{
    core::{
        debug::DebugContext,
        ml::{self, MlModel, MlTask, MlTaskResource},
    },
    vision::camera::{Image, TopImage},
};
use bbox::{Bbox, ConvertBbox, Cxywh, Xyxy};
use box_coder::BoxCoder;
use fast_image_resize as fr;
use itertools::Itertools;
use miette::IntoDiagnostic;
use ndarray::{Array2, Array3, Axis};

mod anchor_generator;
pub mod bbox;
mod box_coder;

use anchor_generator::DefaultBoxGenerator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Inspect)]
#[serde(deny_unknown_fields)]
pub struct RobotDetectionConfig {
    confidence_threshold: f32,
    top_k_detections: usize,
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
        (self.num_anchor_boxes, 4)
    }
}

pub struct RobotDetectionModule;

impl Module for RobotDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_ml_task::<RobotDetectionModel>()?
            .init_config::<RobotDetectionConfig>()?
            .add_startup_system(init_robot_detection)?
            .add_system(detect_robots))
    }
}

/// The robot detection model, based on a VGG-like backbone using SSD detection heads.
pub struct RobotDetectionModel;

impl MlModel for RobotDetectionModel {
    type InputType = f32;
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
}

/// A fitted field boundary from a given image
#[derive(Clone)]
pub struct RobotDetectionData {
    /// The detected robots.
    pub detected: Vec<DetectedRobot>,
    /// The image the robots have been detected in.
    pub image: Image,
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
    };

    storage.add_resource(Resource::new(robot_detection_image))?;
    storage.add_resource(Resource::new(detected_robots))?;

    Ok(())
}

#[system]
fn detect_robots(
    model: &mut MlTask<RobotDetectionModel>,
    config: &RobotDetectionConfig,
    robots: &mut RobotDetectionData,
    robot_detection_image: &mut RobotDetectionImage,
    ctx: &mut DebugContext,
    top_image: &TopImage,
    cycle: &Cycle,
) -> Result<()> {
    if !robot_detection_image.0.is_from_cycle(*cycle) && model.active() {
        // poll for result
        if let Some(result) = model.poll_multi::<Vec<f32>>().transpose()? {
            let box_regression =
                Array2::from_shape_vec(config.box_shape(), result[0].clone()).into_diagnostic()?;
            let scores = Array2::from_shape_vec(config.score_shape(), result[1].clone())
                .into_diagnostic()?;
            let features = Array3::from_shape_vec(config.feature_map_shape, result[2].clone())
                .into_diagnostic()?;

            let threshold = 0.4;
            let detected_robots = postprocess_detections(
                config,
                box_regression,
                scores,
                features,
                threshold,
                config.top_k_detections,
            );

            *robots = RobotDetectionData {
                detected: detected_robots,
                image: robot_detection_image.0.clone(),
            };

            log_detected_robots(robots, ctx)?;
        }

        return Ok(());
    }

    // start new inference
    let resized_image = top_image.resized_yuv(
        config.input_width,
        config.input_height,
        fr::ResizeAlg::Nearest,
    )?;

    let mean_y = 0.4355;
    let mean_u = 0.5053;
    let mean_v = 0.5421;

    let std_y = 0.2713;
    let std_u = 0.0399;
    let std_v = 0.0262;

    if let Ok(()) = model.try_start_infer(
        &resized_image
            .iter()
            .map(|x| *x as f32 / 255.0)
            .enumerate()
            .map(|(i, x)| {
                if i % 3 == 0 {
                    (x - mean_y) / std_y
                } else if (i % 3) == 1 {
                    (x - mean_u) / std_u
                } else {
                    (x - mean_v) / std_v
                }
            })
            .collect::<Vec<f32>>(),
    ) {
        // We need to keep track of the image we started the inference with
        *robot_detection_image = RobotDetectionImage(top_image.deref().clone());
    };

    Ok(())
}

fn postprocess_detections(
    config: &RobotDetectionConfig,
    box_regression: Array2<f32>,
    scores: Array2<f32>,
    features: Array3<f32>,
    threshold: f32,
    k: usize,
) -> Vec<DetectedRobot> {
    let anchor_generator = DefaultBoxGenerator::new(vec![vec![0.4, 0.5], vec![0.85]], 0.15, 0.9);
    let box_coder = BoxCoder::new((10.0, 10.0, 5.0, 5.0));

    let decoded_boxes = box_coder.decode_single(
        box_regression,
        anchor_generator.create_boxes(
            (config.input_width as usize, config.input_height as usize),
            features,
        ),
    );

    scores
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

            // rescale bboxes to 640x480
            let bbox = bbox.scaled(
                640.0 / config.input_width as f32,
                480.0 / config.input_height as f32,
            );

            Some((bbox, scores[1]))
        })
        .sorted_by(|a, b| b.1.total_cmp(&a.1))
        .take(k)
        .map(|(bbox, confidence)| DetectedRobot { bbox, confidence })
        .collect::<Vec<_>>()
}

fn log_detected_robots(robot_data: &RobotDetectionData, ctx: &DebugContext) -> Result<()> {
    let processed_boxes = robot_data
        .detected
        .iter()
        .map(|DetectedRobot { bbox, confidence }| {
            let cxcywh: Bbox<Cxywh> = bbox.convert();
            let (cx, cy, w, h) = cxcywh.into();

            // rerun expects half width and half height
            let half_w = w / 2.0;
            let half_h = h / 2.0;

            (
                ((cx, cy), (half_w, half_h)),
                format!("robot: {confidence:.4}"),
            )
        });

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
