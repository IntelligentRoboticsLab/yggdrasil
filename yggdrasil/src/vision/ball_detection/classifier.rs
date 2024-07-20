use std::ops::Deref;
use std::time::{Duration, Instant};

use heimdall::CameraMatrix;
use itertools::Itertools;
use nalgebra::{Point2, Vector2};

use nidhogg::types::{color, FillExt, LeftEye};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

use crate::localization::RobotPose;
use crate::nao::manager::NaoManager;
use crate::nao::manager::Priority::Medium;
use crate::prelude::*;
use crate::vision::camera::BottomImage;
use crate::vision::camera::{matrix::CameraMatrices, Image, TopImage};

use crate::core::ml::{self, MlModel, MlTask, MlTaskResource};
use crate::vision::scan_lines::CameraType;

use super::proposal::{self, BallProposals, BottomBallProposals, TopBallProposals};
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
        app.add_system(ball_detection_system.after(proposal::ball_proposals_system))
            .add_startup_system(init_ball_classifier)?
            .add_ml_task::<BallClassifierModel>()
    }
}

#[startup_system]
fn init_ball_classifier(
    storage: &mut Storage,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    let balls = Balls {
        balls: Vec::new(),
        top_image: top_image.deref().clone(),
        bottom_image: bottom_image.deref().clone(),
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
    pub scale: f32,
    pub distance: f32,
    pub timestamp: Instant,
    pub confidence: f32,
    pub camera: CameraType,
}

#[derive(Clone)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub top_image: Image,
    pub bottom_image: Image,
}

impl Balls {
    pub fn no_balls(&self) -> bool {
        self.balls.is_empty()
    }

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
pub(super) fn ball_detection_system(
    balls: &mut Balls,
    (top_proposals, bottom_proposals): (&TopBallProposals, &BottomBallProposals),
    model: &mut MlTask<BallClassifierModel>,
    camera_matrices: &CameraMatrices,
    config: &BallDetectionConfig,
    nao: &mut NaoManager,
    robot_pose: &RobotPose,
) -> Result<()> {
    detect_balls(
        top_proposals,
        model,
        balls,
        &camera_matrices.top,
        config,
        robot_pose,
        CameraType::Top,
    )?;

    detect_balls(
        bottom_proposals,
        model,
        balls,
        &camera_matrices.bottom,
        config,
        robot_pose,
        CameraType::Bottom,
    )?;

    if balls.no_balls() {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Medium);
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::PURPLE), Medium);
    }

    Ok(())
}

fn detect_balls(
    proposals: &BallProposals,
    model: &mut MlTask<BallClassifierModel>,
    balls: &mut Balls,
    camera_matrix: &CameraMatrix,
    config: &BallDetectionConfig,
    robot_pose: &RobotPose,
    camera: CameraType,
) -> Result<()> {
    if balls.top_image.is_from_cycle(proposals.image.cycle()) {
        return Ok(());
    }

    let classifier = &config.classifier;
    let start = Instant::now();

    let mut classified_balls = Vec::new();
    'outer: for proposal in proposals
        .proposals
        .iter()
        .sorted_by(|a, b| a.distance_to_ball.total_cmp(&b.distance_to_ball))
    {
        if proposal.distance_to_ball > 20.0 {
            continue;
        }

        let patch_size = proposal.scale as usize;
        let patch = proposals.image.get_grayscale_patch(
            (proposal.position.x, proposal.position.y),
            patch_size,
            patch_size,
        );

        let patch = ml::util::resize_patch(
            (patch_size, patch_size),
            (IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE),
            patch,
        );
        if let Ok(()) = model.try_start_infer(&patch) {
            loop {
                if start.elapsed().as_micros() > classifier.time_budget as u128 {
                    match model.try_cancel() {
                        Ok(()) => (),
                        Err(ml::Error::Tyr(tyr::tasks::Error::NotActive)) => (),
                        Err(e) => {
                            tracing::error!("Failed to cancel ball classifier inference: {:?}", e);
                        }
                    }

                    break 'outer;
                }

                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    let confidence = ml::util::sigmoid(result[0]);
                    if (1.0 - confidence) < classifier.confidence_threshold {
                        break;
                    }

                    if let Ok(robot_to_ball) =
                        camera_matrix.pixel_to_ground(proposal.position.cast(), 0.0)
                    {
                        classified_balls.push(Ball {
                            position_image: proposal.position.cast(),
                            robot_to_ball: robot_to_ball.xy().coords,
                            scale: proposal.scale,
                            position: robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())),
                            distance: proposal.distance_to_ball,
                            timestamp: Instant::now(),
                            confidence: 1.0 - confidence,
                            camera,
                        });

                        if 1.0 - confidence > classifier.confidence_threshold {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    if classified_balls.is_empty() {
        for ball in balls.balls.iter() {
            if ball.timestamp.elapsed() < classifier.ball_life {
                classified_balls.push(ball.clone());
            }
        }
    }

    balls.balls = classified_balls;
    let new_image = proposals.image.clone();
    match camera {
        CameraType::Top => balls.top_image = new_image,
        CameraType::Bottom => balls.bottom_image = new_image,
    };

    Ok(())
}
