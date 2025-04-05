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
use serde_with::serde_as;
use serde_with::DurationMilliSeconds;

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
            .add_systems(PostStartup, (init_ball_tracker.after(init_camera::<Top>),))
            .add_systems(
                Update,
                (
                    classify_balls::<Top>.run_if(resource_exists_and_changed::<BallProposals<Top>>),
                    classify_balls::<Bottom>
                        .run_if(resource_exists_and_changed::<BallProposals<Bottom>>),
                )
                    .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
            );
    }
}

fn init_ball_tracker(mut commands: Commands) {
    //TODO: extract into ball tracker default init.
    let ball_tracker = BallTracker {
        position_kf: UnscentedKalmanFilter::<2, 5, BallPosition>::new(
            BallPosition(Point2::new(0.0, 0.0)),
            CovarianceMatrix::from_diagonal_element(0.05),
        ),
        prediction_noise: CovarianceMatrix::from_diagonal_element(0.01),
        sensor_noise: CovarianceMatrix::from_diagonal_element(1.0),
        cycle: Cycle::default(),
        timestamp: Instant::now(),
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

    let mut confident_balls = Vec::new();

    for proposal in proposals
        .proposals
        .drain(..)
        .sorted_by(|a, b| a.distance_to_ball.total_cmp(&b.distance_to_ball))
    {
        if proposal.distance_to_ball > 20.0 {
            continue;
        }

        if start.elapsed().as_micros() > classifier.time_budget as u128 {
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

        ball_tracker.measurement_update(position);
        confident_balls.push((position, confidence, proposal));

        // TODO: we only store the closest ball with high enough confidence
        // Maybe we should store multiple candidates.
        break;
    }
    // Prediction Update
    ball_tracker.predict();

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

        // Measurement update
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
