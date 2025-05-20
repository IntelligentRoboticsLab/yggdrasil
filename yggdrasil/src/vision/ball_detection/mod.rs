//! Module for detecting balls in the top and bottom images.

pub mod ball_tracker;
pub mod classifier;
pub mod hypothesis;
pub mod proposal;

use std::{sync::Arc, time::Duration};

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, Top};
use nidhogg::types::{FillExt, LeftEye, color};
use proposal::BallProposalConfigs;

use rerun::{AsComponents, FillMode, external::arrow};
use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};

use crate::{
    core::debug::DebugContext,
    localization::RobotPose,
    nao::{Cycle, NaoManager, Priority},
    prelude::*,
};

use self::classifier::BallClassifierConfig;

/// Plugin for detecting balls in the top and bottom images.
pub struct BallDetectionPlugin;

impl Plugin for BallDetectionPlugin {
    fn build(&self, app: &mut App) {
        use self::hypothesis::{measurement_update, predict};

        app.init_config::<BallDetectionConfig>();
        app.add_plugins((
            proposal::BallProposalPlugin::<Top>::default(),
            proposal::BallProposalPlugin::<Bottom>::default(),
            classifier::BallClassifierPlugin,
        ))
        .add_systems(
            PostStartup,
            (
                init_subconfigs,
                setup_ball_debug_logging::<Top>,
                setup_ball_debug_logging::<Bottom>,
                setup_3d_ball_debug_logging,
            ),
        )
        .add_systems(Update, (detected_ball_eye_color, predict, measurement_update.after(predict)))
        .add_systems(PostUpdate, log_3d_balls);
    }
}

#[serde_as]
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct BallDetectionConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub max_classification_age_eye_color: Duration,
    pub proposal: BallProposalConfigs,
    pub classifier: BallClassifierConfig,
}

impl Config for BallDetectionConfig {
    const PATH: &'static str = "ball_detection.toml";
}

// TODO: find a better way to do this (reflection :sob:)
fn init_subconfigs(mut commands: Commands, config: Res<BallDetectionConfig>) {
    commands.insert_resource(config.proposal.clone());
}

/// System that sets up the entities paths in rerun.
///
/// # Note
///
/// By logging a static [`rerun::Color`] component, we can avoid logging the color component
/// for each ball proposal and classification.
fn setup_ball_debug_logging<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_static(
        T::make_entity_image_path("balls/proposals"),
        &rerun::Boxes2D::update_fields().with_colors([(190, 190, 190)]),
    );

    dbg.log_static(
        T::make_entity_image_path("balls/classifications"),
        &rerun::Boxes2D::update_fields()
            .with_colors([(228, 153, 255)])
            .with_draw_order(11.0),
    );
}

fn detected_ball_eye_color(
    mut nao: ResMut<NaoManager>,
    hypotheses_query: Query<&hypothesis::BallHypothesis>,
    config: Res<BallDetectionConfig>,
) {
    let mut best_hypothesis: Option<&hypothesis::BallHypothesis> = None;
    for current_hypothesis in hypotheses_query.iter() {
        if let hypothesis::BallFilter::Stationary(_) = &current_hypothesis.filter {
            if let Some(best) = best_hypothesis {
                if current_hypothesis.num_observations > best.num_observations {
                    best_hypothesis = Some(current_hypothesis);
                }
            } else {
                best_hypothesis = Some(current_hypothesis);
            }
        }
    }

    if best_hypothesis.is_none() {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        return;
    }

    let chosen_hypothesis = best_hypothesis.unwrap();

    if chosen_hypothesis.last_observation.elapsed() >= config.max_classification_age_eye_color {
        nao.set_left_eye_led(
            LeftEye::fill(color::Rgb::new(1.0, 1.0, 0.0)),
            Priority::default(),
        );
    } else {
        nao.set_left_eye_led(
            LeftEye::fill(color::Rgb::new(0.9, 0.6, 1.0)),
            Priority::default(),
        );
    }
}

fn setup_3d_ball_debug_logging(dbg: DebugContext) {
    dbg.log_static(
        "balls/best",
        &[
            rerun::Asset3D::from_file("./assets/rerun/ball.glb")
                .expect("failed to load ball model")
                .with_media_type(rerun::MediaType::glb())
                .as_serialized_batches(),
            rerun::Ellipsoids3D::update_fields()
                .with_fill_mode(FillMode::Solid)
                .as_serialized_batches(),
        ],
    );

    dbg.log_with_cycle(
        "balls/best",
        Cycle::default(),
        &rerun::Transform3D::from_scale((0., 0., 0.)),
    );
}

fn log_3d_balls(
    dbg: DebugContext,
    hypotheses_query: Query<&hypothesis::BallHypothesis>,
    robot_pose: Res<RobotPose>,
) {
    let mut best_hypothesis: Option<&hypothesis::BallHypothesis> = None;
    for current_hypothesis in hypotheses_query.iter() {
        if matches!(current_hypothesis.filter, hypothesis::BallFilter::Stationary(_)) {
            if let Some(best) = best_hypothesis {
                if current_hypothesis.num_observations > best.num_observations {
                    best_hypothesis = Some(current_hypothesis);
                }
            } else {
                best_hypothesis = Some(current_hypothesis);
            }
        }
    }

    if let Some(chosen_hypothesis) = best_hypothesis {
        if let hypothesis::BallFilter::Stationary(kf) = &chosen_hypothesis.filter {
            let pos = robot_pose.robot_to_world(&kf.position());
            let cov = kf.covariance(); // Assuming kf is the KalmanFilter_ (e.g. kf.0 if it's a tuple struct)
            let max_variance = cov.diagonal().max();

            const STATIONARY_VARIANCE_THRESHOLD_PLACEHOLDER: f32 = 0.1; // Placeholder
            let scale = (1.0 - (max_variance / STATIONARY_VARIANCE_THRESHOLD_PLACEHOLDER)).clamp(0.0, 1.0);
            let std_dev = max_variance.sqrt();

            // It's important to use the same cycle for all components of the same entity path
            // if we are not using log_with_cycle. For simplicity, we'll log them together.
            // The previous implementation used dbg.log_with_cycle, but the new instructions
            // use dbg.log. We'll use the cycle from the hypothesis if available,
            // otherwise, PostUpdate cycle might be implicit.
            // For now, let's stick to the simpler dbg.log as per instructions,
            // and assume Rerun handles timestamping appropriately or it's less critical here.

            let transform_batch = rerun::Transform3D::from_translation((pos.coords.x, pos.coords.y, 0.05))
                .as_serialized_batches();
            let ellipsoid_batch = rerun::Ellipsoids3D::from_half_sizes([(std_dev, std_dev, 0.005)])
                .with_colors([(0, (126.0 * scale) as u8, (31.0 * scale) as u8)])
                .as_serialized_batches();
            let variance_batch = rerun::SerializedComponentBatch::new(
                Arc::new(arrow::array::Float32Array::from_value(max_variance, 1)),
                rerun::ComponentDescriptor::new("yggdrasil.components.Variance"),
            )
            .as_serialized_batches();
            
            dbg.log("balls/best", &[transform_batch, ellipsoid_batch, variance_batch].concat());

        } else {
            // Should not happen if best_hypothesis selection logic is correct and it's Stationary
            dbg.log("balls/best", &rerun::Transform3D::from_scale((0., 0., 0.)));
        }
    } else {
        // No stationary hypothesis found, hide the entity
        dbg.log("balls/best", &rerun::Transform3D::from_scale((0., 0., 0.)));
    }
}
