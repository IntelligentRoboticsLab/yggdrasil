//! Module for detecting the referee pose

mod detect;
pub mod recognize;

use bevy::prelude::*;
use detect::{RefereePoseDetected, RefereePoseDetectionPlugin};
use recognize::{RecognizeRefereePose, RefereePoseRecognitionPlugin};

pub struct VisualRefereePlugin;

impl Plugin for VisualRefereePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RefereePoseDetectionPlugin, RefereePoseRecognitionPlugin))
            .add_event::<RecognizeRefereePose>()
            .init_state::<VisualRefereeDetectionStatus>()
            .add_systems(
                Update,
                (
                    activate_detection_status
                        .run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
                    deactivate_detection_status
                        .run_if(in_state(VisualRefereeDetectionStatus::Active)),
                ),
            );
    }
}

fn activate_detection_status(
    mut detect_pose: EventReader<DetectRefereePose>,
    mut next_detection_status: ResMut<NextState<VisualRefereeDetectionStatus>>,
) {
    for _ev in detect_pose.read() {
        next_detection_status.set(VisualRefereeDetectionStatus::Active);
        break;
    }
}

fn deactivate_detection_status(
    mut pose_detected: EventReader<RefereePoseDetected>,
    mut next_detection_status: ResMut<NextState<VisualRefereeDetectionStatus>>,
) {
    for _ev in pose_detected.read() {
        next_detection_status.set(VisualRefereeDetectionStatus::Inactive);
        break;
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisualRefereeDetectionStatus {
    #[default]
    Inactive,
    Active,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Event)]
pub struct DetectRefereePose;
