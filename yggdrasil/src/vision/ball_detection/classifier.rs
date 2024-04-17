use std::num::NonZeroU32;
use std::ops::Deref;
use std::time::Instant;

use fast_image_resize as fr;

use nalgebra::{Point2, Point3};
use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use crate::camera::matrix::CameraMatrices;
use crate::camera::{Image, TopImage};
use crate::debug::DebugContext;
use crate::prelude::*;

use crate::ml::{MlModel, MlTask, MlTaskResource};

use super::proposal::BallProposals;
use super::BallDetectionConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    pub confidence_threshold: f32,
    pub patch_scale: f32,
    pub time_budget: usize,
}

pub(crate) struct BallClassifierModule;

impl Module for BallClassifierModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(detect_balls.after(super::proposal::get_proposals))
            .add_startup_system(init_ball_classifier)?
            .add_ml_task::<BallClassifierModel>()
    }
}

#[startup_system]
fn init_ball_classifier(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    // Initialize the field boundary with a single line at the top of the image
    let balls = Balls {
        balls: Vec::new(),
        image: top_image.deref().clone(),
    };

    storage.add_resource(Resource::new(balls))?;

    Ok(())
}

struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
    type InputType = f32;
    type OutputType = f32;
}

#[derive(Debug, Clone, Default)]
pub struct Ball {
    pub position_image: Point2<f32>,
    pub robot_to_ball: Point3<f32>,
    pub distance: f32,
}

#[derive(Clone)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub image: Image,
}

#[system]
fn detect_balls(
    proposals: &BallProposals,
    model: &mut MlTask<BallClassifierModel>,
    balls: &mut Balls,
    camera_matrices: &CameraMatrices,
    ctx: &DebugContext,
    config: &BallDetectionConfig,
) -> Result<()> {
    if balls.image.timestamp() == proposals.image.timestamp() {
        return Ok(());
    }

    let config = &config.classifier;
    let start = Instant::now();

    let mut classified_balls = Vec::new();
    'outer: for proposal in proposals.proposals.iter() {
        let patch_size = (config.patch_scale / proposal.distance_to_ball) as usize;
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            patch_size,
            patch_size,
        );

        let patch = resize_patch(patch_size, patch_size, patch);
        if let Ok(()) = model.try_start_infer(&patch) {
            loop {
                if start.elapsed().as_micros() > config.time_budget as u128 {
                    if let Err(e) = model.try_cancel() {
                        tracing::error!("Failed to cancel  ball classifier inference: {:?}", e);
                    }

                    break 'outer;
                }

                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    let confidence = result[0];
                    if confidence > config.confidence_threshold {
                        if let Ok(robot_to_ball) = camera_matrices
                            .top
                            .pixel_to_ground(proposal.position.cast(), 0.0)
                        {
                            classified_balls.push(Ball {
                                position_image: proposal.position.cast(),
                                robot_to_ball,
                                distance: proposal.distance_to_ball,
                            });
                        }
                    }

                    break;
                }
            }
        }
    }

    balls.image = proposals.image.clone();
    balls.balls = classified_balls;

    let ball_positions = balls
        .balls
        .iter()
        .map(|ball| (ball.position_image.x, ball.position_image.y))
        .collect::<Vec<_>>();
    let amount = ball_positions.len();

    ctx.log_boxes_2d(
        "/top_camera/image/detected_balls",
        ball_positions,
        vec![(32.0, 32.0); amount],
        &balls.image,
        color::u8::RED,
    )?;

    Ok(())
}

/// Resize yuyv image to correct input shape
fn resize_patch(width: usize, height: usize, patch: Vec<u8>) -> Vec<f32> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(width as u32).unwrap(),
        NonZeroU32::new(height as u32).unwrap(),
        patch,
        fr::PixelType::U8,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(32).unwrap(),
        NonZeroU32::new(32).unwrap(),
        src_image.pixel_type(),
    );

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    // Remove every second y value from the yuyv image
    dst_image
        .buffer()
        .iter()
        .map(|p| *p as f32 / 255.0)
        .collect()
}
