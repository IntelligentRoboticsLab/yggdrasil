//! See [`BallClassifierPlugin`].

use std::time::{Duration, Instant};

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix, Top};
use itertools::Itertools;
use ml::prelude::ModelExecutor;
use nalgebra::Point2;

use serde::{Deserialize, Serialize};
use serde_with::{DurationMicroSeconds, serde_as};

use crate::core::debug::DebugContext;

use crate::nao::Cycle;
use crate::vision::referee::detect::VisualRefereeDetectionStatus;
use ml::prelude::*;

use super::BallDetectionConfig;
use super::proposal::BallProposals;

const IMAGE_INPUT_SIZE: usize = 32;

#[serde_as]
#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct BallClassifierConfig {
    /// Minimum confidence score threshold for accepting a ball detection
    pub confidence_threshold: f32,

    /// The amount of time in microseconds we allow the classifier to run, proposals that take longer are discarded.
    #[serde_as(as = "DurationMicroSeconds<u64>")]
    pub time_budget: Duration,
}

/// Plugin for classifying ball proposals produced by [`super::proposal::BallProposalPlugin`].
///
/// This plugin uses a cnn model to classify whether the proposals are balls or not.
pub struct BallClassifierPlugin;

impl Plugin for BallClassifierPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<BallClassifierModel>().add_systems(
            Update,
            (
                classify_balls::<Top>.run_if(resource_exists_and_changed::<BallProposals<Top>>),
                classify_balls::<Bottom>
                    .run_if(resource_exists_and_changed::<BallProposals<Bottom>>),
            )
                .chain()
                .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
        );
    }
}

pub(super) struct BallClassifierModel;

impl MlModel for BallClassifierModel {
    type Inputs = Vec<u8>;
    type Outputs = f32;

    const ONNX_PATH: &'static str = "models/ball_classifier.onnx";
}

#[derive(Clone, Component, Debug)]
pub struct BallPerception {
    /// Ball position relative to the robot
    pub position: Point2<f32>,
    pub cycle: Cycle,
}

#[allow(clippy::too_many_arguments)]
fn classify_balls<T: CameraLocation>(
    ctx: DebugContext,
    cycle: Res<Cycle>,
    mut commands: Commands,
    mut proposals: ResMut<BallProposals<T>>,
    mut model: ResMut<ModelExecutor<BallClassifierModel>>,
    camera_matrix: Res<CameraMatrix<T>>,
    config: Res<BallDetectionConfig>,
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
            .spawn_blocking(ml::util::sigmoid);

        if confidence < classifier.confidence_threshold {
            continue;
        }

        let Ok(robot_to_ball) = camera_matrix.pixel_to_ground(proposal.position.cast(), 0.0) else {
            tracing::warn!(?proposal.position, "failed to project ball position to ground");
            continue;
        };

        let position = robot_to_ball.xy();

        commands.spawn(BallPerception {
            position,
            cycle: *cycle,
        });

        confident_balls.push((confidence, proposal.clone()));

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
        let (confidence, proposal) = confident_balls
            .iter()
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap();

        let (x1, y1, x2, y2) = proposal.bbox.inner;

        ctx.log_with_cycle(
            T::make_entity_image_path("balls/classifications"),
            proposals.image.cycle(),
            &rerun::Boxes2D::from_mins_and_sizes([(x1, y1)], [(x2 - x1, y2 - y1)])
                .with_labels([format!("{confidence:.2}")]),
        );
    }
}
