//! Module for detecting balls in the top and bottom images.

pub mod ball_tracker;
pub mod classifier;
pub mod proposal;

use std::{sync::Arc, time::Duration};

pub use ball_tracker::BallHypothesis;
use ball_tracker::BallTracker;
use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, Top};
use nidhogg::types::{color, FillExt, LeftEye};
use proposal::BallProposalConfigs;

use rerun::{external::arrow, AsComponents, FillMode};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

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
        .add_systems(Update, detected_ball_eye_color)
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
    ball_tracker: Res<BallTracker>,
    config: Res<BallDetectionConfig>,
) {
    let Some(_) = ball_tracker.stationary_ball() else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        return;
    };

    if ball_tracker.timestamp.elapsed() >= config.max_classification_age_eye_color {
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
    ball_tracker: Res<BallTracker>,
    robot_pose: Res<RobotPose>,
    mut last_logged: Local<Option<Cycle>>,
) {
    let last_ball_tracker_update = ball_tracker.cycle;
    let state = ball_tracker.cutoff();

    if let BallHypothesis::Stationary(max_variance) = state {
        let pos = robot_pose.robot_to_world(&ball_tracker.state());
        if last_logged.is_none_or(|last_logged_cycle| last_ball_tracker_update > last_logged_cycle)
        {
            *last_logged = Some(last_ball_tracker_update);
            let std = max_variance.sqrt();
            let scale =
                1.0 - (max_variance / ball_tracker.stationary_variance_threshold).clamp(0.0, 1.0);

            dbg.log_with_cycle(
                "balls/best",
                last_ball_tracker_update,
                &[
                    rerun::Transform3D::from_translation((pos.coords.x, pos.coords.y, 0.05))
                        .as_serialized_batches(),
                    rerun::Ellipsoids3D::from_half_sizes([(std, std, 0.005)])
                        .with_colors([(0, (126.0 * scale) as u8, (31.0 * scale) as u8)])
                        .as_serialized_batches(),
                    rerun::SerializedComponentBatch::new(
                        Arc::new(arrow::array::Float32Array::from_value(max_variance, 1)),
                        rerun::ComponentDescriptor::new("yggdrasil.components.Variance"),
                    )
                    .as_serialized_batches(),
                ],
            );
        }
    } else if last_logged.is_some() {
        // this feels very hacky but i was told this is the most idiomatic way to hide stuff in
        // rerun.
        *last_logged = None;
        dbg.log("balls/best", &rerun::Transform3D::from_scale((0., 0., 0.)));
    }
}
