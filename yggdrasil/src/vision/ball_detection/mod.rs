//! Module for detecting the location of the ball in the field

pub mod classifier;
pub mod proposal;

use std::time::Duration;

use nidhogg::types::{color, FillExt, LeftEye};
use proposal::BallProposalConfig;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{
    core::debug::DebugContext,
    nao::manager::{NaoManager, Priority},
    prelude::*,
    vision::camera::matrix::CameraMatrices,
};

use self::{
    classifier::{BallClassifierConfig, Balls},
    proposal::BallProposals,
};

pub struct BallDetectionModule;

impl Module for BallDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(proposal::BallProposalModule)?
            .add_module(classifier::BallClassifierModule)?
            .add_system(log_balls.after(classifier::detect_balls))
            .add_system(reset_eye_color.after(classifier::detect_balls))
            .init_inspectable_config::<BallDetectionConfig>()?
            .add_startup_system(init_subconfigs)
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Inspect)]
pub struct BallDetectionConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub max_classification_age_eye_color: Duration,
    pub proposal: BallProposalConfig,
    pub classifier: BallClassifierConfig,
}

impl Config for BallDetectionConfig {
    const PATH: &'static str = "ball_detection.toml";
}

// TODO: find a better way to do this (reflection :sob:)
#[startup_system]
fn init_subconfigs(storage: &mut Storage, config: &mut BallDetectionConfig) -> Result<()> {
    storage.add_resource(Resource::new(config.proposal.clone()))?;

    Ok(())
}

#[system]
fn log_balls(
    dbg: &DebugContext,
    ball_proposals: &BallProposals,
    balls: &Balls,
    matrices: &CameraMatrices,
    config: &BallProposalConfig,
) -> Result<()> {
    let mut points = Vec::new();
    let mut sizes = Vec::new();
    for proposal in &ball_proposals.proposals {
        // project point to ground to get distance
        // distance is used for the amount of surrounding pixels to sample
        let Ok(coord) = matrices.top.pixel_to_ground(proposal.position.cast(), 0.0) else {
            continue;
        };

        let magnitude = coord.coords.magnitude();

        let size = config.bounding_box_scale / magnitude;

        points.push((proposal.position.x as f32, proposal.position.y as f32));
        sizes.push((size, size));
    }

    dbg.log_boxes_2d(
        "top_camera/image/ball_boxes",
        &points.clone(),
        &sizes,
        &ball_proposals.image,
        color::u8::SILVER,
    )?;

    dbg.log_points2d_for_image_with_radius(
        "top_camera/image/ball_spots",
        &points,
        ball_proposals.image.cycle(),
        color::u8::GREEN,
        4.0,
    )?;

    let mut positions = Vec::new();
    let mut sizes = Vec::new();
    for ball in &balls.balls {
        positions.push((ball.position_image.x, ball.position_image.y));
        let size = config.bounding_box_scale / ball.distance;
        sizes.push((size, size));
    }

    dbg.log_boxes_2d(
        "top_camera/image/detected_balls",
        &positions,
        &sizes,
        &balls.image,
        color::u8::PURPLE,
    )?;

    Ok(())
}

#[system]
fn reset_eye_color(
    balls: &Balls,
    nao: &mut NaoManager,
    config: &BallDetectionConfig,
) -> Result<()> {
    let best_ball = balls.most_recent_ball();
    if let Some(ball) = best_ball {
        if ball.timestamp.elapsed() >= config.max_classification_age_eye_color {
            nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
        }
    } else {
        nao.set_left_eye_led(LeftEye::fill(color::f32::EMPTY), Priority::default());
    }

    Ok(())
}
