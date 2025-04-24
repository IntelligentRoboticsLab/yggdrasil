use bevy::prelude::*;

use super::{
    RefereePose, RefereePoseConfig,
    detect::{DetectRefereePose, RefereePoseDetected},
};

pub struct RefereePoseRecognitionPlugin;

impl Plugin for RefereePoseRecognitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RefereePoseRecognized>()
            .init_state::<VisualRefereeRecognitionStatus>()
            .init_resource::<DetectedRefereePoses>()
            .add_systems(
                Update,
                (
                    request_recognition,
                    recognizing_pose.run_if(in_state(VisualRefereeRecognitionStatus::Active)),
                )
                    .chain(),
            );
    }
}

/// Recognize a referee pose.
///
/// # Arguments
/// - `detected_poses` ([`DetectedRefereePoses`]) keeps track of earlier detected
///   referee poses.
/// - `detected_pose` ([`RefereePoseDetected`]) is an event that is received when
///   a pose detection is finished.
/// - `detect_pose` ([`DetectRefereePose`]) is an event that is send when a pose
///   detection is requested.
/// - `recognized_pose` ([`RefereePoseRecognized`]) is an event that is send when
///   enough poses are detected in a sequence
pub fn recognizing_pose(
    mut detected_poses: ResMut<DetectedRefereePoses>,
    mut detected_pose: EventReader<RefereePoseDetected>,
    mut detect_pose: EventWriter<DetectRefereePose>,
    mut recognized_pose: EventWriter<RefereePoseRecognized>,
    mut next_recognition_status: ResMut<NextState<VisualRefereeRecognitionStatus>>,
    referee_pose_config: Res<RefereePoseConfig>,
) {
    for pose in detected_pose.read() {
        let recognition_config = &referee_pose_config.recognition;
        // Check whether we detected VISUAL_REFEREE_DETECT_ATTEMPTS number of times.
        if detected_poses.poses.len() < recognition_config.referee_consecutive_pose_detections {
            // Add detected pose to vector remember
            detected_poses.poses.push(pose.pose);
            // Resend a request to detect a new referee pose
            detect_pose.send(DetectRefereePose);
        } else {
            // Determine if pose was the same
            if let Some(pose) = all_same_poses(&detected_poses.poses) {
                // Send final pose recognition
                recognized_pose.send(RefereePoseRecognized { pose: *pose });
            }
            // Deactivate the visual referee recogition state
            next_recognition_status.set(VisualRefereeRecognitionStatus::Inactive);
            // Empty the memory of previous detected states
            detected_poses.clear();
        }
    }
}

/// Starts the recognition of a referee pose when a request is received from
/// [`RecognizeRefereePose`].
///
/// It first activates the [`VisualRefereeRecognitionStatus`] state and starts the
/// detection pose chain by sending an request to detect a referee pose via the
/// [`DetectRefereePose`] event.
pub fn request_recognition(
    mut recognize_pose: EventReader<RecognizeRefereePose>,
    mut next_recognition_status: ResMut<NextState<VisualRefereeRecognitionStatus>>,
    mut detect_pose: EventWriter<DetectRefereePose>,
) {
    if recognize_pose.read().last().is_some() {
        next_recognition_status.set(VisualRefereeRecognitionStatus::Active);
        // Send the initil request to detect a referee pose
        detect_pose.send(DetectRefereePose);
    }
}

/// [`DetectedRefereePoses`] functions as memory for detected referee poses
#[derive(Resource, Default)]
pub struct DetectedRefereePoses {
    poses: Vec<RefereePose>,
}

impl DetectedRefereePoses {
    /// Clears the memory of earlier detected referee poses
    pub fn clear(&mut self) {
        self.poses.clear();
    }
}

/// A bevy state ([`States`]), which keeps track of whether the referee pose reconition
/// is ongoing or not.
#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisualRefereeRecognitionStatus {
    #[default]
    Inactive,
    Active,
}

/// A bevy [`Event`] that requests to start recognizing a referee pose
#[derive(Event)]
pub struct RecognizeRefereePose;

/// A bevy [`Event`] for when a referee pose is recognized
#[derive(Event)]
pub struct RefereePoseRecognized {
    pub pose: RefereePose,
}

// Determines whether all poses are the same
fn all_same_poses(poses: &[RefereePose]) -> Option<&RefereePose> {
    let all_same = poses
        .first()
        .is_none_or(|first| poses.iter().all(|x| x == first));

    if all_same { poses.first() } else { None }
}
