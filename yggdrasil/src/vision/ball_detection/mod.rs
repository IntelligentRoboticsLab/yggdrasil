//! Module for detecting the location of the ball in the field

pub mod classifier;
pub mod proposal;

use std::time::Duration;

use nidhogg::types::{color, FillExt, LeftEye};
use proposal::BallProposalConfigs;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

use crate::{
    core::debug::DebugContext,
    nao::manager::{NaoManager, Priority},
    prelude::*,
};

use self::classifier::{BallClassifierConfig, Balls};

use super::scan_lines::CameraType;

pub struct BallDetectionModule;

impl Module for BallDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(proposal::BallProposalModule)?
            .add_module(classifier::BallClassifierModule)?
            .add_system(log_balls.after(classifier::ball_detection_system))
            .add_system(reset_eye_color.after(classifier::ball_detection_system))
            .init_config::<BallDetectionConfig>()?
            .add_startup_system(init_subconfigs)
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[startup_system]
fn init_subconfigs(storage: &mut Storage, config: &mut BallDetectionConfig) -> Result<()> {
    storage.add_resource(Resource::new(config.proposal.clone()))?;

    Ok(())
}

#[system]
fn log_balls(balls: &Balls, dbg: &DebugContext) -> Result<()> {
    let mut positions_top = Vec::new();
    let mut sizes_top = Vec::new();

    let mut positions_bottom = Vec::new();
    let mut sizes_bottom = Vec::new();

    for ball in &balls.balls {
        let pos = (ball.position_image.x, ball.position_image.y);
        let size = (ball.scale / 2.0, ball.scale / 2.0);

        match ball.camera {
            CameraType::Top => {
                positions_top.push(pos);
                sizes_top.push(size);
            }
            CameraType::Bottom => {
                positions_bottom.push(pos);
                sizes_bottom.push(size);
            }
        };
    }

    dbg.log_boxes2d_with_class(
        "top_camera/image/detected_balls",
        &positions_top,
        &sizes_top,
        balls
            .balls
            .iter()
            .filter(|x| x.camera == CameraType::Top)
            .map(|b| format!("{:.3}", b.confidence))
            .collect(),
        balls.top_image.cycle(),
    )?;

    dbg.log_boxes2d_with_class(
        "bottom_camera/image/detected_balls",
        &positions_bottom,
        &sizes_bottom,
        balls
            .balls
            .iter()
            .filter(|x| x.camera == CameraType::Bottom)
            .map(|b| format!("{:.3}", b.confidence))
            .collect(),
        balls.bottom_image.cycle(),
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
