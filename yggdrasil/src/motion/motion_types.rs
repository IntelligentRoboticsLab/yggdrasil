use crate::motion::motion_util::lerp;
use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::JointArray;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSecondsWithFrac};
use serde_json;
use std::fs::File;
use std::{path::Path, time::Duration};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a single robot movement.
pub struct Movement {
    /// Movement target joint positions.
    pub target_position: JointArray<f32>,
    /// Movement duration.
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub duration: Duration,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a robot motion that consists of multiple movements.
pub struct Motion {
    /// Joint starting positions for the motion.
    pub initial_position: JointArray<f32>,
    /// Vector containing movements needed to reach the final position.
    pub movements: Vec<Movement>,
}

impl Motion {
    /// Initializes a motion from a motion file. Uses serde for deserialization.
    ///
    /// # Arguments
    ///
    /// * `path` - the `Path` to the file from which to read the motion.
    pub fn from_path(path: &Path) -> Result<Motion> {
        match serde_json::from_reader(File::open(path).into_diagnostic()?) {
            Ok(val) => Ok(val),
            Err(err) => Err(miette! {
               "Could not deserialize json {}: {}", path.display(), err
            }),
        }
    }

    /// Retrieves the a target position for each joint by using linear
    /// interpolation between the two nearest positions based on the starting
    /// time and current time.
    ///
    /// # Arguments
    ///
    /// * `motion_duration` - Duration of the current motion.
    pub fn get_position(&self, motion_duration: Duration) -> Option<JointArray<f32>> {
        self.get_surrounding_frames_as_joint_array(motion_duration)
            .map(|(target_positions_a, target_positions_b, duration)| {
                lerp(
                    &target_positions_a,
                    &target_positions_b,
                    motion_duration.as_secs_f32() / duration.as_secs_f32(),
                )
            })
    }

    /// Get the nearest position that the robot should have before the
    /// duration and the nearest position after the duration and the total
    /// duration of the corresponding motion.
    ///
    /// # Arguments
    ///
    /// * `motion_duration` - Current duration of the current motion.
    fn get_surrounding_frames_as_joint_array(
        &self,
        motion_duration: Duration,
    ) -> Option<(&JointArray<f32>, &JointArray<f32>, &Duration)> {
        for (i, movement) in self.movements.iter().enumerate() {
            if motion_duration <= movement.duration && i < self.movements.len() {
                let start_position = if i == 0 {
                    &self.initial_position
                } else {
                    &self.movements[i - 1].target_position
                };
                let target_position = &movement.target_position;
                let duration = &movement.duration;

                return Some((start_position, target_position, duration));
            }
        }

        None
    }
}

/// An enumeration of all possible motions.
#[derive(PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum MotionType {
    Example,
    FallForwards,
    FallBackwards,
    FallLeftways,
    FallRightways,
    Neutral,
}
