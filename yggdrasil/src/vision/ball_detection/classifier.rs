use std::ops::Deref;
use std::time::{Duration, Instant};

use nalgebra::{Point2, Vector2};

use nidhogg::types::{color, FillExt, LeftEye};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

use crate::localization::RobotPose;
use crate::nao::manager::NaoManager;
use crate::nao::manager::Priority::Medium;
use crate::prelude::*;
use crate::vision::camera::{matrix::CameraMatrices, Image, TopImage};

use crate::core::ml::{MlModel, MlTask, MlTaskResource};

use super::proposal::BallProposals;
use super::BallDetectionConfig;

const IMAGE_INPUT_SIZE: usize = 32;

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    pub confidence_threshold: f32,
    pub time_budget: usize,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub ball_life: Duration,
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

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
    type InputType = f32;
    type OutputType = f32;
}

#[derive(Debug, Clone)]
pub struct Ball {
    pub position_image: Point2<f32>,
    pub robot_to_ball: Vector2<f32>,
    pub position: Point2<f32>,
    pub distance: f32,
    pub timestamp: Instant,
    pub confidence: f32,
}

#[derive(Clone)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub image: Image,
}

impl Balls {
    pub fn most_confident_ball(&self) -> Option<&Ball> {
        self.balls
            .iter()
            .reduce(|a, b| match a.confidence > b.confidence {
                true => a,
                false => b,
            })
    }

    pub fn most_recent_ball(&self) -> Option<&Ball> {
        self.balls
            .iter()
            .reduce(|a, b| match a.timestamp > b.timestamp {
                true => a,
                false => b,
            })
    }
}

#[system]
pub(super) fn detect_balls(
    proposals: &BallProposals,
    model: &mut MlTask<BallClassifierModel>,
    balls: &mut Balls,
    camera_matrices: &CameraMatrices,
    config: &BallDetectionConfig,
    nao: &mut NaoManager,
    robot_pose: &RobotPose,
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
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            patch_size,
            patch_size,
        );

        let patch = crate::core::ml::util::resize_patch(
            (patch_size, patch_size),
            (IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE),
            patch,
        );
        if let Ok(()) = model.try_start_infer(&patch) {
            loop {
                if start.elapsed().as_micros() > classifier.time_budget as u128 {
                    match model.try_cancel() {
                        Ok(()) => (),
                        Err(crate::core::ml::Error::Tyr(tyr::tasks::Error::NotActive)) => (),
                        Err(e) => {
                            tracing::error!("Failed to cancel ball classifier inference: {:?}", e);
                        }
                    }

                    break 'outer;
                }

                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    let confidence = result[0];
                    if confidence < classifier.confidence_threshold {
                        break;
                    }

                    if let Ok(robot_to_ball) = camera_matrices
                        .top
                        .pixel_to_ground(proposal.position.cast(), 0.0)
                    {
                        classified_balls.push(Ball {
                            position_image: proposal.position.cast(),
                            robot_to_ball: robot_to_ball.xy().coords,
                            position: robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())),
                            distance: proposal.distance_to_ball,
                            timestamp: Instant::now(),
                            confidence,
                        });
                    }
                }
            }
        }
    }

    if !classified_balls.is_empty() {
        nao.set_left_eye_led(LeftEye::fill(color::f32::PURPLE), Medium);
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Medium);
    }

    balls.image = proposals.image.clone();
    if classified_balls.is_empty() {
        for ball in balls.balls.iter() {
            if ball.timestamp.elapsed() < classifier.ball_life {
                classified_balls.push(ball.clone());
            }
        }
    }

    balls.balls = classified_balls;

    Ok(())
}
