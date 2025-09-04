//! Module for detecting balls in the top and bottom images.

pub mod classifier;
pub mod hypothesis;
pub mod proposal;

use std::time::Duration;

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nidhogg::types::{FillExt, LeftEye, color};
use proposal::BallProposalConfigs;

use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};

use crate::{
    nao::{NaoManager, Priority},
    prelude::*,
    vision::ball_detection::hypothesis::Ball,
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
            hypothesis::BallHypothesisPlugin,
        ))
        .add_systems(PostStartup, (init_subconfigs,))
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

fn detected_ball_eye_color(
    mut nao: ResMut<NaoManager>,
    ball: Res<Ball>,
    config: Res<BallDetectionConfig>,
) {
    let Ball::Some(ref ball) = *ball else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        return;
    };

    if ball.last_update.elapsed() >= config.max_classification_age_eye_color {
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
