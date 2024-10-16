//! See [`BallClassifierPlugin`].

use std::time::{Duration, Instant};

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix, CameraPosition, Top};
use itertools::Itertools;
use ml::prelude::ModelExecutor;
use nalgebra::{Point2, Vector2};

use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

use crate::core::debug::DebugContext;
use crate::localization::RobotPose;

use crate::nao::Cycle;
use crate::vision::camera::init_camera;
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

/// Plugin for classifying ball proposals produced by [`super::proposal::BallProposalPlugin`].
///
/// This plugin uses a cnn model to classify whether the proposals are balls or not.
pub struct BallClassifierPlugin;

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
                    detect_balls::<Top>.run_if(resource_exists_and_changed::<BallProposals<Top>>),
                    detect_balls::<Bottom>
                        .run_if(resource_exists_and_changed::<BallProposals<Bottom>>),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    log_ball_classifications::<Top>.run_if(resource_exists_and_changed::<Balls>),
                    log_ball_classifications::<Bottom>.run_if(resource_exists_and_changed::<Balls>),
                ),
            );
    }
}

fn init_ball_classifier(mut commands: Commands) {
    let balls = Balls {
        balls: Vec::new(),
        cycle: Cycle::default(),
    };

    commands.insert_resource(balls);
}

fn log_ball_classifications<T: CameraLocation>(dbg: DebugContext, balls: Res<Balls>) {
    if balls.balls.is_empty() {
        return;
    }
    let (positions, (half_sizes, confidences)): (Vec<_>, (Vec<_>, Vec<_>)) = balls
        .balls
        .iter()
        // TODO: Once we have a better unified way to store the balls for different camera positions, we can
        // drop the extra condition here.
        .filter(|ball| ball.is_fresh && ball.camera == T::POSITION)
        .map(|ball| {
            (
                (ball.position_image.x, ball.position_image.y),
                (
                    (ball.scale / 2.0, ball.scale / 2.0),
                    format!("{:.2}", ball.confidence),
                ),
            )
        })
        .unzip();

    if positions.is_empty() {
        return;
    }

    dbg.log_with_cycle(
        T::make_entity_path("balls/classifications"),
        balls.cycle,
        &rerun::Boxes2D::from_centers_and_half_sizes(&positions, &half_sizes)
            .with_labels(confidences),
    );
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
    /// Whether this detection is from the most recent frame.
    pub is_fresh: bool,
    pub camera: CameraPosition,
}

#[derive(Clone, Resource)]
pub struct Balls {
    pub balls: Vec<Ball>,
    pub cycle: Cycle,
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
            is_fresh: true,
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
                // keep the ball if it isn't too old, but mark it as not fresh
                classified_balls.push(Ball {
                    is_fresh: false,
                    ..ball.clone()
                });
            }
        }
    }

    balls.balls = classified_balls;
    balls.cycle = proposals.image.cycle();
}
