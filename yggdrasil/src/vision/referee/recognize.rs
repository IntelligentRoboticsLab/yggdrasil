use bevy::prelude::*;

use super::{detect::RefereePoseDetected, DetectRefereePose, RefereePose};

// TODO: Probably in a config
const VISUAL_REFEREE_DETECT_ATTEMPTS: usize = 5;

pub struct RefereePoseRecognitionPlugin;

impl Plugin for RefereePoseRecognitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RefereePoseRecognised>()
            .init_state::<VisualRefereeRecognitionStatus>()
            .init_resource::<DetectedRefereePoses>()
            .add_systems(
                Update,
                (
                    request_recognition,
                    recognising_pose.run_if(in_state(VisualRefereeRecognitionStatus::Active)),
                    show_recognized_pose,
                ),
            );
    }
}

pub fn recognising_pose(
    mut detected_poses: ResMut<DetectedRefereePoses>,
    mut detected_pose: EventReader<RefereePoseDetected>,
    mut detect_pose: EventWriter<DetectRefereePose>,
    mut recognized_pose: EventWriter<RefereePoseRecognised>,
    mut next_recognition_status: ResMut<NextState<VisualRefereeRecognitionStatus>>,
) {
    for pose in detected_pose.read() {
        if detected_poses.poses.len() < VISUAL_REFEREE_DETECT_ATTEMPTS {
            // Add detected pose to vector remember
            detected_poses.poses.push(pose.pose.clone());
            // Resend a request to detect a new referee pose
            detect_pose.send(DetectRefereePose);
        } else {
            // Determine if pose was the same
            if all_same(&detected_poses.poses) {
                let pose = detected_poses.poses.first().expect("Does not happen :)");
                // Send final pose recognition
                recognized_pose.send(RefereePoseRecognised { pose: pose.clone() });
            }
            // Deactivate the visual referee recogition state
            next_recognition_status.set(VisualRefereeRecognitionStatus::Inactive);
            // Empty the memory of previous detected states
            detected_poses.clear();
        }
    }
}

/// Starts the recognition of a post.
/// Activates the `VisualRefereeRecognitionStatus` state and starts the
/// detection pose chain by sending an initial `DetectRefereePose` event.
pub fn request_recognition(
    mut recognise_pose: EventReader<RecognizeRefereePose>,
    mut next_recognition_status: ResMut<NextState<VisualRefereeRecognitionStatus>>,
    mut detect_pose: EventWriter<DetectRefereePose>,
) {
    for _ev in recognise_pose.read() {
        next_recognition_status.set(VisualRefereeRecognitionStatus::Active);
        // Send the initil request to detect a referee pose
        detect_pose.send(DetectRefereePose);
        break;
    }
    recognise_pose.clear();
}

#[derive(Resource, Default)]
pub struct DetectedRefereePoses {
    poses: Vec<RefereePose>,
}

impl DetectedRefereePoses {
    pub fn clear(&mut self) {
        self.poses.clear();
    }
}

#[derive(Event)]
pub struct RecognizeRefereePose;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisualRefereeRecognitionStatus {
    #[default]
    Inactive,
    Active,
}

#[derive(Event)]
pub struct RefereePoseRecognised {
    pub pose: RefereePose,
}

pub fn show_recognized_pose(mut recognized_pose: EventReader<RefereePoseRecognised>) {
    for pose in recognized_pose.read() {
        println!("Pose recognized: {:?}", pose.pose)
    }
}

fn all_same(poses: &[RefereePose]) -> bool {
    poses
        .first()
        .map(|first| poses.iter().all(|x| x == first))
        .unwrap_or(true)
}
