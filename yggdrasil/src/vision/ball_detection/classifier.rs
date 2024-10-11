use std::ops::Deref;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix, CameraPosition, Top};
use itertools::Itertools;
use ml::prelude::ModelExecutor;
use nalgebra::{Point2, Vector2};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

use crate::localization::RobotPose;

use crate::vision::camera::{init_camera, Image};
use ml::prelude::*;

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

pub(crate) struct BallClassifierPlugin;

impl Plugin for BallClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<BallClassifierModel>()
            .add_systems(
                PostStartup,
                init_ball_classifier
                    .after(init_camera::<Top>)
                    .after(init_camera::<Bottom>),
            )
            .add_systems(
                Update,
                (
                    detect_balls::<Top>
                        .after(super::proposal::update_ball_proposals::<Top>)
                        .run_if(resource_exists_and_changed::<Image<Top>>),
                    detect_balls::<Bottom>
                        .after(super::proposal::update_ball_proposals::<Bottom>)
                        .run_if(resource_exists_and_changed::<Image<Bottom>>),
                ),
            );
    }
}

fn init_ball_classifier(
    mut commands: Commands,
    top_image: Res<Image<Top>>,
    bottom_image: Res<Image<Bottom>>,
) {
    let balls = Balls {
        balls: Vec::new(),
        top_image: top_image.deref().clone(),
        bottom_image: bottom_image.deref().clone(),
    };

    commands.insert_resource(balls);
}

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";

    type InputElem = f32;
    type OutputElem = f32;

    type InputShape = (Vec<f32>,);
    type OutputShape = (Vec<f32>,);
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
    pub camera: CameraPosition,
}

#[derive(Clone, Resource)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub top_image: Image<Top>,
    pub bottom_image: Image<Bottom>,
}

impl Balls {
    #[must_use]
    pub fn no_balls(&self) -> bool {
        self.balls.is_empty()
    }

    #[must_use]
    pub fn most_confident_ball(&self) -> Option<&Ball> {
        self.balls
            .iter()
            .reduce(|a, b| if a.confidence > b.confidence { a } else { b })
    }

    #[must_use]
    pub fn most_recent_ball(&self) -> Option<&Ball> {
        self.balls
            .iter()
            .reduce(|a, b| if a.timestamp > b.timestamp { a } else { b })
    }
}

fn detect_balls<T: CameraLocation>(
    mut commands: Commands,
    mut proposals: ResMut<BallProposals<T>>,
    mut model: ResMut<ModelExecutor<BallClassifierModel>>,
    mut balls: ResMut<Balls>,
    camera_matrix: Res<CameraMatrix<T>>,
    config: Res<BallDetectionConfig>,
    robot_pose: Res<RobotPose>,
) {
    let classifier = &config.classifier;
    let start = Instant::now();

    let mut classified_balls = Vec::new();

    for proposal in proposals
        .proposals
        .drain(..)
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

        let confidence = {
            let output = commands
                .infer_model(&mut model)
                .with_input(&(patch,))
                .spawn_blocking(|(result,)| ml::util::sigmoid(result[0]))[0];

            1.0 - output
        };

        if start.elapsed().as_micros() > classifier.time_budget as u128 {
            break;
        }

        if confidence < classifier.confidence_threshold {
            continue;
        }

        let Ok(robot_to_ball) = camera_matrix.pixel_to_ground(proposal.position.cast(), 0.0) else {
            continue;
        };

        classified_balls.push(Ball {
            position_image: proposal.position.cast(),
            robot_to_ball: robot_to_ball.xy().coords,
            scale: proposal.scale,
            position: robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())),
            distance: proposal.distance_to_ball,
            timestamp: Instant::now(),
            confidence,
            camera: T::POSITION,
        });

        // TODO: we only store the closest ball with high enough confidence
        // Maybe we should store multiple candidates.
        break;
    }

    if classified_balls.is_empty() {
        for ball in &balls.balls {
            if ball.timestamp.elapsed() < classifier.ball_life {
                classified_balls.push(ball.clone());
            }
        }
    }

    balls.balls = classified_balls;
}
