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
    nao::{NaoManager, Priority},
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
            ),
        )
        .add_systems(Update, detected_ball_eye_color);
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
    dbg.log_component_batches(
        T::make_entity_path("balls/proposals"),
        true,
        [&rerun::Color::from_rgb(190, 190, 190) as _],
    );
    dbg.log_component_batches(
        T::make_entity_path("balls/classifications"),
        true,
        [&rerun::Color::from_rgb(128, 0, 128) as _],
    );
}

fn detected_ball_eye_color(
    mut nao: ResMut<NaoManager>,
    balls: Res<Balls>,
    config: Res<BallDetectionConfig>,
) {
    let best_ball = balls.most_recent_ball();

    if let Some(ball) = best_ball {
        if ball.timestamp.elapsed() >= config.max_classification_age_eye_color {
            nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        } else {
            nao.set_left_eye_led(LeftEye::fill(color::f32::PURPLE), Priority::default());
        }
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
    }
}
