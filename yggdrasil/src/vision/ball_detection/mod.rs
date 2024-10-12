//! Module for detecting balls in the top and bottom images.

pub mod classifier;
pub mod proposal;
mod visualizer;

use std::time::Duration;

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nidhogg::types::{color, FillExt, LeftEye};
use proposal::BallProposalConfigs;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{
    nao::{NaoManager, Priority},
    prelude::*,
};

use self::classifier::{BallClassifierConfig, Balls};

/// Plugin for detecting balls in the top and bottom images.
pub struct BallDetectionPlugin;

impl Plugin for BallDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<BallDetectionConfig>();
        app.add_systems(PostStartup, init_subconfigs);

        app.add_plugins((
            proposal::BallProposalPlugin::<Top>::default(),
            proposal::BallProposalPlugin::<Bottom>::default(),
            classifier::BallClassifierPlugin,
            visualizer::BallDetectionVisualizerPlugin::<Top>::default(),
            visualizer::BallDetectionVisualizerPlugin::<Bottom>::default(),
        ));

        app.add_systems(Update, detected_ball_eye_color);
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
