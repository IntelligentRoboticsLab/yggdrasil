use std::marker::PhantomData;
use std::time::Instant;

use bevy::prelude::*;
use ml::{prelude::*, util::PatchResizer};
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

use filter::{CovarianceMatrix, UnscentedKalmanFilter};
use heimdall::{Bottom, CameraLocation, CameraMatrix, Top};
use nalgebra::Point2;

use crate::{
    core::debug::DebugContext,
    localization::odometry::Odometry,
    nao::Cycle,
    vision::{
        ball_detection::proposal::{BallProposal, BallProposals},
        camera::init_camera,
        referee::detect::VisualRefereeDetectionStatus,
    },
};

use super::{
    BallDetectionConfig,
    ball_tracker::{BallPosition, BallTracker},
};

const IMAGE_INPUT_SIZE: usize = 32;

/// Plugin for classifying ball proposals using a neural network.
pub struct BallClassifierPlugin;

impl Plugin for BallClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<BallClassifierModel>()
            .add_systems(PostStartup, init_ball_tracker.after(init_camera::<Top>))
            .add_systems(
                Update,
                (
                    update_ball_tracker,
                    dispatch_ball_classification::<Top>
                        .run_if(task_finished::<BallClassification<Top>>)
                        .run_if(|p: Res<BallProposals<Top>>| !p.proposals.is_empty()),
                    handle_ball_classification_result::<Top>
                        .run_if(resource_exists_and_changed::<BallClassification<Top>>),
                    dispatch_ball_classification::<Bottom>
                        .run_if(task_finished::<BallClassification<Bottom>>)
                        .run_if(|p: Res<BallProposals<Bottom>>| !p.proposals.is_empty()),
                    handle_ball_classification_result::<Bottom>
                        .run_if(resource_exists_and_changed::<BallClassification<Bottom>>),
                )
                    .chain()
                    .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
            );
    }
}

fn init_ball_tracker(mut commands: Commands, config: Res<BallDetectionConfig>) {
    let config = &config.classifier;

    commands.insert_resource(BallTracker {
        position_kf: UnscentedKalmanFilter::<2, 5, BallPosition>::new(
            BallPosition(Point2::origin()),
            CovarianceMatrix::from_diagonal_element(config.stationary_std_threshold.powi(2)), // variance = std^2, and we don't know where the ball is
        ),
        // prediction is done each cycle, this is roughly 1.7cm of std per cycle or 1.3 meters per second
        prediction_noise: CovarianceMatrix::from_diagonal_element(config.prediction_noise),
        sensor_noise: CovarianceMatrix::from_diagonal_element(config.measurement_noise),
        cycle: Cycle::default(),
        timestamp: Instant::now(),
        stationary_variance_threshold: config.stationary_std_threshold.powi(2),
    });
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    /// Minimum confidence score threshold for accepting a ball detection
    pub confidence_threshold: f32,

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

#[derive(Resource, Debug, Clone)]
pub struct BallClassification<T: CameraLocation + Clone> {
    pub proposal: BallProposal,
    pub confidence: f32,
    pub cycle: Cycle, // image cycle the patch came from
    _marker: PhantomData<T>,
}

impl<T: CameraLocation + Clone> BallClassification<T> {
    /// Was the net confident enough?
    #[inline]
    fn is_confident(&self, thresh: f32) -> bool {
        self.confidence >= thresh
    }
}

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    type Inputs = Vec<u8>;
    type Outputs = f32;
    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
}

fn update_ball_tracker(mut ball_tracker: ResMut<BallTracker>, odometry: Res<Odometry>) {
    ball_tracker.predict(&odometry);
}

fn dispatch_ball_classification<T: CameraLocation + Clone>(
    mut commands: Commands,
    mut proposals: ResMut<BallProposals<T>>,
    mut model: ResMut<ModelExecutor<BallClassifierModel>>,
    mut patch_resizer: Local<Option<PatchResizer>>,
) {
    // nothing left to classify
    if proposals.proposals.is_empty() {
        return;
    }

    let resizer = patch_resizer
        .get_or_insert_with(|| PatchResizer::new(IMAGE_INPUT_SIZE as u32, IMAGE_INPUT_SIZE as u32));

    // pick the "best" proposal (closest ball = smallest distance)
    let idx_best = proposals
        .proposals
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.distance_to_ball.total_cmp(&b.distance_to_ball))
        .map(|(i, _)| i)
        .unwrap();

    // remove it from the queue so we won't touch it again
    let proposal = proposals.proposals.remove(idx_best);
    let patch_size = proposal.scale as usize;
    let patch = proposals.image.get_grayscale_patch(
        (proposal.position.x, proposal.position.y),
        patch_size,
        patch_size,
    );

    resizer.resize_patch(&patch, (patch_size, patch_size));

    let cycle = proposals.image.cycle();

    commands
        .infer_model(&mut model)
        .with_input(&resizer.take())
        .create_resource()
        .spawn(move |raw_output| {
            let confidence = ml::util::sigmoid(raw_output);

            Some(BallClassification::<T> {
                proposal,
                confidence,
                cycle,
                _marker: PhantomData,
            })
        });
}

fn handle_ball_classification_result<T: CameraLocation + Clone>(
    ctx: DebugContext,
    classification: Res<BallClassification<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
    mut ball_tracker: ResMut<BallTracker>,
    config: Res<BallDetectionConfig>,
) {
    let result = classification.clone();

    // confidence gate
    if result.is_confident(config.classifier.confidence_threshold) {
        // project pixel to ground
        if let Ok(robot_to_ball) =
            camera_matrix.pixel_to_ground(result.proposal.position.cast(), 0.0)
        {
            // UKF measurement update
            ball_tracker.measurement_update(BallPosition(robot_to_ball.xy()));
        }

        // log accepted detection
        let (x1, y1, x2, y2) = result.proposal.bbox.inner;
        ctx.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            result.cycle,
            &rerun::Boxes2D::from_mins_and_sizes([(x1, y1)], [(x2 - x1, y2 - y1)])
                .with_labels([format!("{:.2}", result.confidence)]),
        );
    } else {
        // not confident, clear log in Rerun
        ctx.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            result.cycle,
            &rerun::Clear::flat(),
        );
    }

    // keep tracker cycle in sync
    ball_tracker.cycle = result.cycle;
}
