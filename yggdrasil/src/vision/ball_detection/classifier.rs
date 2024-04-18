use std::ops::Deref;
use std::time::Instant;

use nalgebra::{Point2, Point3, Vector2};
use ndarray::Array;
use nidhogg::types::{color, FillExt, LeftEye};
use serde::{Deserialize, Serialize};

use crate::camera::matrix::CameraMatrices;
use crate::camera::{Image, TopImage};
use crate::debug::DebugContext;
use crate::nao::manager::NaoManager;
use crate::nao::manager::Priority::Medium;
use crate::prelude::*;

use crate::ml::{MlModel, MlTask, MlTaskResource};

use super::proposal::BallProposals;
use super::BallDetectionConfig;

const IMAGE_INPUT_SIZE: usize = 32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    pub confidence_threshold: f32,
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
    pub robot_to_ball: Vector2<f32>,
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
    nao: &mut NaoManager,
) -> Result<()> {
    if balls.image.timestamp() == proposals.image.timestamp() {
        return Ok(());
    }

    let classifier = &config.classifier;
    let start = Instant::now();

    let mut classified_balls = Vec::new();
    'outer: for proposal in proposals.proposals.iter() {
        if proposal.distance_to_ball > 20.0 {
            continue;
        }

        let patch_size = proposal.scale as usize;
        tracing::info!(
            "scale: {}, distance: {}, patch_size: {}",
            proposal.scale,
            proposal.distance_to_ball,
            patch_size
        );
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            patch_size,
            patch_size,
        );

        let patch = crate::ml::util::resize_patch(
            (patch_size, patch_size),
            (IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE),
            patch,
        );
        if let Ok(()) = model.try_start_infer(&patch) {
            loop {
                if start.elapsed().as_micros() > classifier.time_budget as u128 {
                    if let Err(e) = model.try_cancel() {
                        tracing::error!("Failed to cancel ball classifier inference: {:?}", e);
                    }

                    break 'outer;
                }

                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    let confidence = result[0];
                    if confidence > 0.4 {
                        ctx.log_patch(
                            "/patch/",
                            balls.image.cycle(),
                            Array::from_shape_vec((32, 32, 1), patch).unwrap(),
                        )?;
                    }

                    tracing::info!(
                        "confidence: {} >= {}",
                        confidence,
                        classifier.confidence_threshold
                    );
                    if confidence >= classifier.confidence_threshold {
                        tracing::info!("ball with conf");
                        if let Ok(robot_to_ball) = camera_matrices
                            .top
                            .pixel_to_ground(proposal.position.cast(), 0.0)
                        {
                            tracing::info!("BALL! with pos");
                            classified_balls.push(Ball {
                                position_image: proposal.position.cast(),
                                robot_to_ball: robot_to_ball.xy().coords,
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
        &ball_positions,
        &vec![(32.0, 32.0); amount],
        &balls.image,
        color::u8::RED,
    )?;

    if !balls.balls.is_empty() {
        nao.set_left_eye_led(LeftEye::fill(color::f32::PURPLE), Medium);
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Medium);
    }

    Ok(())
}
