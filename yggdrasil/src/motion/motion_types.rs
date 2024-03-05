use crate::motion::motion_util::lerp;
use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::JointArray;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::fs::File;
use std::{path::Path, time::Duration};

use toml;

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
                    target_positions_a,
                    target_positions_b,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InterpolationType {
    Linear,
    SmoothIn,
    SmoothOut,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConditionalVariable {
    GyroscopeX,
    GyroscopeY,
    AngleX,
    AngleY,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionCondition {
    pub variable: ConditionalVariable,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionConfig {
    // CURRENTLY NOT USED
    pub interpolation_type: InterpolationType,
    // CURRENTLY NOT USED
    pub wait_time: f32,
    pub submotions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubMotion {
    pub joint_stifness: f32,
    pub chest_angle_bound_upper: f32,
    pub chest_angle_bound_lower: f32,
    pub conditions: MotionCondition,
    pub keyframes: Vec<Movement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComplexMotion {
    pub motion_config: MotionConfig,
    pub submotions: Vec<SubMotion>,
}

impl ComplexMotion {
    /// Initializes a motion from a motion file. Uses serde for deserialization.
    ///
    /// # Arguments
    ///
    /// * `path` - the `Path` to the file from which to read the motion.
    // pub fn from_path(path: &Path) -> Result<Motion> {
    //     match serde_json::from_reader(File::open(path).into_diagnostic()?) {
    //         Ok(val) => Ok(val),
    //         Err(err) => Err(miette! {
    //            "Could not deserialize json {}: {}", path.display(), err
    //         }),
    //     }
    // }
    pub fn from_path(path: &Path) -> Result<ComplexMotion> {
        // checking whether the specified complex motion file has been generated
        if !path.exists() {
            // if not, we generate it based on the existing config file
            let toml_path = path.with_extension(".toml");
            let motion_config_data = std::fs::read_to_string(toml_path).into_diagnostic()?;
            let config: MotionConfig = toml::de::from_str(&motion_config_data).into_diagnostic()?;

            // "./assets/motions/submotions"
            // use the above root file path along with the submotion names to deserialize the submotions and put them into the complexmotion struct

            // based on the gathered config file, we not generate a new ComplexMotion
            let mut complexmotion: ComplexMotion = ComplexMotion {
                motion_config: config.clone(),
                submotions: Vec::new(),
            };

            // populating the submotions property of ComplexMotion with the correct SubMotions
            for submotion_name in config.submotions.iter() {
                let submotion_path = Path::new("./assets/motions/submotions")
                    .join(submotion_name)
                    .with_extension(".json");
                let submotion: SubMotion =
                    serde_json::from_reader(File::open(submotion_path).into_diagnostic()?)
                        .expect("Reading Submotion file during ComplexMotion construction");
                complexmotion.submotions.push(submotion)
            }

            // when the ComplexMotion has been created, we save it to the assets/motions folder
            serde_json::to_writer(
                &File::create("data.json").into_diagnostic()?,
                &complexmotion,
            )
            .into_diagnostic()?;

            return Ok(complexmotion);
        } else {
            // if the json file for the ComplexMotion does exist, simply deserialize and return it
            match serde_json::from_reader(File::open(path).into_diagnostic()?) {
                Ok(val) => Ok(val),
                Err(err) => Err(miette! {
                   "Could not deserialize json {}: {}", path.display(), err
                }),
            }
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
                    target_positions_a,
                    target_positions_b,
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
        // TODO IMPLEMENT SURROUNDING FRAMES FUNCTION

        // for (i, movement) in self.movements.iter().enumerate() {
        //     if motion_duration <= movement.duration && i < self.movements.len() {
        //         let start_position = if i == 0 {
        //             &self.initial_position
        //         } else {
        //             &self.movements[i - 1].target_position
        //         };
        //         let target_position = &movement.target_position;
        //         let duration = &movement.duration;

        //         return Some((start_position, target_position, duration));
        //     }
        // }

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
