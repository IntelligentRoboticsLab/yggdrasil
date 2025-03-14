//! Module for recognizing a referee pose.
//!
//! The process of recognizing a referee pose starts by requesting the recognition via
//! the [`RecognizeRefereePose`] event.
//!
//! We make a distinction between "recognition" and "detection".
//! - Recognition: This is the final prediction for a pose. A pose is "recognized" based on
//!   multiple pose detections
//!
//! - Detection: A single referee pose prediction. For the referee pose this is a single
//!   inference with the [`RefereePoseDetectionModel`](crate::vision::referee::detect::RefereePoseDetectionModel)

pub mod communication;
pub mod detect;
pub mod recognize;

use bevy::prelude::*;
use bifrost::serialization::{Decode, Encode};
use communication::RefereePoseCommunicationPlugin;
use detect::RefereePoseDetectionPlugin;
use odal::Config;
use recognize::{RecognizeRefereePose, RefereePoseRecognitionPlugin};
use serde::{Deserialize, Serialize};

use crate::prelude::ConfigExt;

/// This plugin ([`Plugin`]) handles the detection and recognition of the referee pose.
pub struct VisualRefereePlugin;

impl Plugin for VisualRefereePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RefereePoseDetectionPlugin,
            RefereePoseRecognitionPlugin,
            RefereePoseCommunicationPlugin,
        ))
        .init_config::<RefereePoseConfig>()
        .add_event::<RecognizeRefereePose>();
    }
}

/// The referee poses that can be detected by a robot
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq)]
pub enum RefereePose {
    /// Class 0. Combination of `Idle`, `PlayerExchangeBlue`, `PlayerExchangeRed`,
    /// `FullTime`
    Idle,
    /// Class 1. Combination of `GoalKickRed`, `GoalKickBlue`
    GoalKick,
    /// Class 2. Combination of `GoalBlue`, `GoalRed`
    Goal,
    /// Class 3. Combination of `PushingFreeKickBlue`, `PushingFreeKickRed`
    PushingFreeKick,
    /// Class 4. Combination of `CornerKickBlue`, `CornerKickRed`
    CornerKick,
    /// Class 5. Combination of `KickInBlue`, `KickInRed`
    KickIn,
    /// Class 6.
    Ready,
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct RefereePoseConfig {
    detection: RefereePoseDetectionConfig,
    recognition: RefereePoseRecognitionConfig,
}

impl Config for RefereePoseConfig {
    const PATH: &'static str = "referee_pose.toml";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefereePoseDetectionConfig {
    crop_width: u32,
    crop_height: u32,
    input_width: u32,
    input_height: u32,
    keypoints_shape: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefereePoseRecognitionConfig {
    referee_consecutive_pose_detections: usize,
}
