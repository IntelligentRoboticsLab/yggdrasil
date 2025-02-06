//! Module for detecting balls in the top and bottom images.

pub mod classifier;
pub mod proposal;

use std::time::Duration;

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, Top};
use nidhogg::types::{color, FillExt, LeftEye};
use proposal::BallProposalConfigs;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{
    core::debug::DebugContext,
    nao::{Cycle, NaoManager, Priority},
    prelude::*,
};

use self::classifier::{BallClassifierConfig, Balls};

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
    bottom_balls: Res<Balls<Bottom>>,
    top_balls: Res<Balls<Top>>,
    config: Res<BallDetectionConfig>,
) {
    let best_ball = bottom_balls
        .most_recent_ball()
        .map(|b| b.timestamp)
        .or(top_balls.most_recent_ball().map(|b| b.timestamp));

    if let Some(timestamp) = best_ball {
        if timestamp.elapsed() >= config.max_classification_age_eye_color {
            nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        } else {
            nao.set_left_eye_led(
                LeftEye::fill(color::Rgb::new(0.9, 0.6, 1.0)),
                Priority::default(),
            );
        }
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
    }
}

fn setup_3d_ball_debug_logging(dbg: DebugContext) {
    dbg.log_static(
        "balls/best",
        &rerun::Asset3D::from_file("./assets/rerun/ball.glb")
            .expect("failed to load ball model")
            .with_media_type(rerun::MediaType::glb()),
    );

    dbg.log_with_cycle(
        "balls/best",
        Cycle::default(),
        &rerun::Transform3D::from_scale((0., 0., 0.)),
    );
}

fn log_3d_balls(
    dbg: DebugContext,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    mut last_logged: Local<Option<Cycle>>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| (b.cycle, b.position))
        .or(top_balls
            .most_confident_ball()
            .map(|b| (b.cycle, b.position)));

    if let Some((cycle, pos)) = most_confident_ball {
        // since we always run this function in the same cycle in which the ball was found,
        // using `log_with_cycle` would be redundant.
        //
        // because `last_logged` starts off at 0, we wouldn't log the ball if it was detected in
        // the very first cycle, i don't really care tho, deal with it.
        if last_logged.map_or(true, |c| cycle > c) {
            *last_logged = Some(cycle);
            dbg.log(
                "balls/best",
                &rerun::Transform3D::from_translation((pos.coords.x, pos.coords.y, 0.05)),
            );
        }
    } else if last_logged.is_some() {
        // this feels very hacky but i was told this is the most idiomatic way to hide stuff in
        // rerun.
        *last_logged = None;
        dbg.log("balls/best", &rerun::Transform3D::from_scale((0., 0., 0.)));
    }
}
