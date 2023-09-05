use crate::motion::motion_executer::lerp;
use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::JointArray;
use serde::Deserialize;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::fs::File;
use std::{path::Path, time::Duration};

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
/// Represents a single robot movement.
pub struct Movement {
    /// Movement target joint positions.
    pub target_position: JointArray<f32>,
    /// Movement duration.
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub duration: Duration,
}

#[derive(Deserialize, Debug, Clone)]
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

    /// Retrieves the current position by using linear interpolation between the
    /// two nearest positions based on the starting time and current time.
    ///
    /// # Arguments
    ///
    /// * `motion_duration` - Duration of the current motion.
    pub fn get_position(&self, motion_duration: Duration) -> Option<JointArray<f32>> {
        self.get_surrounding_frames(motion_duration)
            .map(|(frame_a, frame_b)| {
                lerp(
                    &frame_a.target_position,
                    &frame_b.target_position,
                    motion_duration.as_secs_f32()
                        / (frame_b.duration - frame_a.duration).as_secs_f32(),
                )
            })
    }

    /// Get the nearest position that the robot should have before the
    /// duration and the nearest position after the duration.
    ///
    /// # Arguments
    ///
    /// * `motion_duration` - Duration of the current motion.
    fn get_surrounding_frames(&self, motion_duration: Duration) -> Option<(&Movement, &Movement)> {
        for (i, movement) in self.movements.iter().enumerate() {
            if motion_duration >= movement.duration && i < self.movements.len() - 1 {
                return Some((&self.movements[i], &self.movements[i + 1]));
            }
        }

        None
    }
}

/// An enumeration of all possible motions.
#[derive(PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum MotionType {
    Test,
    SitDownFromStand,
    StandUpFromSit,
}
