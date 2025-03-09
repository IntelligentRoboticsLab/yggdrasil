//! Module for detecting the referee pose

mod classifier;
mod estimator;

use bevy::prelude::*;
use classifier::{RefereePoseClassifierPlugin, RefereePoseDetected};
use estimator::RefereePoseEstimatorPlugin;

pub struct VisualRefereePlugin;

impl Plugin for VisualRefereePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RefereePoseEstimatorPlugin, RefereePoseClassifierPlugin))
            .init_state::<VisualRefereeDetectionStatus>()
            .add_systems(
                Update,
                (
                    activate_detection_status.run_if(in_state(VisualRefereeDetectionStatus::Inactive)),
                    deactivate_detection_status.run_if(in_state(VisualRefereeDetectionStatus::Active)),
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

#[derive(Clone, Debug)]
pub enum RefereePose {
    Idle,
    PlayerExchangeBlue,
    GoalKickRed,
    GoalBlue,
    GoalKickBlue,
    PushingFreeKickBlue,
    CornerKickBlue,
    PushingFreeKickRed,
    KickInBlue,
    PlayerExchangeRed,
    GoalRed,
    KickInRed,
    CornerKickRed,
    FullTime,
}

#[derive(Event)]
pub struct DetectRefereePose;
