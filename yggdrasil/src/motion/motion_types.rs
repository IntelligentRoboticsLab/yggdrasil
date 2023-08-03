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
    pub target_positions: JointArray<f32>,
    /// Movement duration.
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub duration: Duration,
}

#[derive(Deserialize, Debug, Clone)]
/// Represents a robot motion that consists of multiple movements.
pub struct Motion {
    /// Joint starting positions for the motion.
    pub initial_positions: JointArray<f32>,
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

    /// Get the frames that surround the elapsed time.
    ///
    /// # Arguments
    ///
    /// * `motion_duration` - Current duration of the motion.
    pub fn get_surrounding_frames(
        &self,
        motion_duration: &Duration,
    ) -> Option<(&Movement, &Movement)> {
        for (i, movement) in self.movements.iter().enumerate() {
            if *motion_duration >= movement.duration && i < self.movements.len() - 1 {
                return Some((&self.movements[i], &self.movements[i + 1]));
            }
        }

        None
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
/// An enumeration of all possible motions.
pub enum MotionType {
    SitDownFromStand,
    StandUpFromSit,
}
