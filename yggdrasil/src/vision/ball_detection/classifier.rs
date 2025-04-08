//! See [`BallClassifierPlugin`].

use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use filter::{CovarianceMatrix, UnscentedKalmanFilter};
use heimdall::{Bottom, CameraLocation, CameraMatrix, Top};
use itertools::Itertools;
use ml::prelude::ModelExecutor;
use nalgebra::{Point2, Vector2};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMicroSeconds};

use crate::core::debug::DebugContext;
use crate::localization::RobotPose;

use crate::nao::Cycle;
use crate::vision::camera::init_camera;
use crate::vision::referee::detect::VisualRefereeDetectionStatus;
use ml::prelude::*;

use super::ball_tracker::{BallPosition, BallTracker};
use super::proposal::BallProposals;
use super::BallDetectionConfig;

const IMAGE_INPUT_SIZE: usize = 32;

#[serde_as]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    /// Minimum confidence score threshold for accepting a ball detection
    pub confidence_threshold: f32,

    /// The amount of time in microseconds we allow the classifier to run, proposals that take longer are discarded.
    #[serde_as(as = "DurationMicroSeconds<u64>")]
    pub time_budget: Duration,

    /// Process noise parameter for position prediction in the Kalman filter
    pub prediction_noise: f32,

    /// Measurement noise parameter for the Kalman filter
    pub measurement_noise: f32,

    /// Maximum standard deviation threshold for stationary ball detection in the Kalman filter.
    ///
    /// Values below this threshold indicate the ball is stationary, while values above indicate movement.
    ///
    /// # Note:
    /// Currently we only track stationary balls, not moving ones.
    pub stationary_std_threshold: f32,
}

/// Plugin for classifying ball proposals produced by [`super::proposal::BallProposalPlugin`].
///
/// This plugin uses a cnn model to classify whether the proposals are balls or not.
pub struct BallClassifierPlugin;

impl Plugin for BallClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<BallClassifierModel>()
            .add_systems(PostStartup, (init_ball_tracker.after(init_camera::<Top>),))
            .add_systems(
                Update,
                (
                    update_ball_tracker, // prediction step should run once each cycle
                    classify_balls::<Top>.run_if(resource_exists_and_changed::<BallProposals<Top>>),
                    classify_balls::<Bottom>
                        .run_if(resource_exists_and_changed::<BallProposals<Bottom>>),
                )
                    .chain()
                    .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
            );
    }
}

fn init_ball_tracker(mut commands: Commands, config: Res<BallDetectionConfig>) {
    let config = config.classifier.clone();

    let ball_tracker = BallTracker {
        position_kf: UnscentedKalmanFilter::<2, 5, BallPosition>::new(
            BallPosition(Point2::new(0.0, 0.0)),
            CovarianceMatrix::from_diagonal_element(config.stationary_std_threshold.powi(2)), // variance = std^2, and we don't know where the ball is
        ),
        // prediction is done each cycle, this is roughly 1.7cm of std per cycle or 1.3 meters per second
        prediction_noise: CovarianceMatrix::from_diagonal_element(config.prediction_noise),
        sensor_noise: CovarianceMatrix::from_diagonal_element(config.measurement_noise),
        cycle: Cycle::default(),
        timestamp: Instant::now(),
        stationary_variance_threshold: config.stationary_std_threshold.powi(2), // variance = std^2
    };

    commands.insert_resource(ball_tracker);
}

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    type Inputs = Vec<f32>;
    type Outputs = f32;

    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
}

#[derive(Debug)]
pub struct Ball<T: CameraLocation> {
    /// The position of the ball proposal in the image, in pixels.
    pub position_image: Point2<f32>,
    /// The vector from the robot to the ball proposal, in robot frame.
    pub robot_to_ball: Vector2<f32>,
    /// The absolute position of the ball proposal, in world frame.
    pub position: Point2<f32>,
    /// Velocity of the ball proposal, in world frame.
    // pub velocity: Vector2<f32>,
    /// The scale of the ball proposal.
    pub scale: f32,
    /// The distance to the ball in meters, at the time of detection.
    pub distance: f32,
    /// The timestamp the ball was detected at.
    pub timestamp: Instant,
    /// The confidence score assigned to the detected ball.
    pub confidence: f32,
    /// The cycle of the image this ball was detected in.
    pub cycle: Cycle,
    _marker: PhantomData<T>,
}

// NOTE: This needs to be implemented manually because of the `PhantomData`
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for Ball<T> {
    fn clone(&self) -> Self {
        Self {
            position_image: self.position_image,
            robot_to_ball: self.robot_to_ball,
            position: self.position,
            // velocity: self.velocity,
            scale: self.scale,
            distance: self.distance,
            timestamp: self.timestamp,
            confidence: self.confidence,
            cycle: self.cycle,
            _marker: PhantomData,
        }
    }
}

/// System that runs the prediction step for the UKF backing the ball tracker.
fn update_ball_tracker(mut ball_tracker: ResMut<BallTracker>) {
    ball_tracker.predict();
}

#[allow(clippy::too_many_arguments)]
fn classify_balls<T: CameraLocation>(
    ctx: DebugContext,
    cycle: Res<Cycle>,
    mut commands: Commands,
    mut proposals: ResMut<BallProposals<T>>,
    mut model: ResMut<ModelExecutor<BallClassifierModel>>,
    mut ball_tracker: ResMut<BallTracker>,
    camera_matrix: Res<CameraMatrix<T>>,
    config: Res<BallDetectionConfig>,
    robot_pose: Res<RobotPose>,
) {
    let classifier = &config.classifier;
    let start = Instant::now();

    let sorted_proposals = proposals
        .proposals
        .drain(..)
        .filter(|p| p.distance_to_ball <= 20.0)
        .sorted_by(|a, b| a.distance_to_ball.total_cmp(&b.distance_to_ball))
        .collect::<Vec<_>>();

    let mut confident_balls = Vec::new();

    for proposal in sorted_proposals {
        if start.elapsed() > classifier.time_budget {
            break;
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

        // sigmoid is applied in model onnx
        let confidence = commands
            .infer_model(&mut model)
            .with_input(&patch)
            .spawn_blocking(|output| 1.0 - output);

        if confidence < classifier.confidence_threshold {
            continue;
        }

        let Ok(robot_to_ball) = camera_matrix.pixel_to_ground(proposal.position.cast(), 0.0) else {
            tracing::warn!(?proposal.position, "failed to project ball position to ground");
            continue;
        };

        let position = BallPosition(robot_pose.robot_to_world(&Point2::from(robot_to_ball.xy())));

        confident_balls.push((position, confidence, proposal.clone()));

        // We only store the closest ball with high enough confidence
        break;
    }

    if confident_balls.is_empty() {
        ctx.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            *cycle,
            &rerun::Clear::flat(),
        );
    } else {
        let (best_position, confidence, proposal) = confident_balls
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();

        ball_tracker.measurement_update(*best_position);

        let (x1, y1, x2, y2) = proposal.bbox.inner;

        ctx.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            proposals.image.cycle(),
            &rerun::Boxes2D::from_mins_and_sizes([(x1, y1)], [(x2 - x1, y2 - y1)])
                .with_labels([format!("{confidence:.2}")]),
        );
    }

    ball_tracker.cycle = proposals.image.cycle();
}
