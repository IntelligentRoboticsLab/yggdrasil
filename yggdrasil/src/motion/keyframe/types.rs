use super::interpolate_jointarrays;
use bevy::prelude::*;
use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::JointArray;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::{
    collections::HashMap,
    fs::File,
    path::Path,
    time::{Duration, Instant},
};
use toml;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents a single robot movement.
pub struct Movement {
    pub target_position: JointArray<f32>,
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    pub duration: Duration,
}

/// An enum containing the possible interpolation types for a motion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InterpolationType {
    Linear,
    EaseInOut,
    EaseIn,
    EaseOut,
}

/// Stores information about the different chosen motion settings.
///
/// # Notes
/// - New motion settings should be added here as a new property.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionSettings {
    pub interpolation_type: InterpolationType,
    pub motion_order: Vec<String>,
}

/// Stores information about a submotion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubMotion {
    pub joint_stifness: f32,
    pub keyframes: Vec<Movement>,
}

/// Stores information about a motion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Motion {
    pub settings: MotionSettings,
    pub submotions: HashMap<String, SubMotion>,
}

impl Motion {
    /// Initializes a motion from a motion config file. Uses serde for deserialization.
    /// Generates the appropriate motion file from a motion config file.
    ///
    /// # Arguments
    /// * `path` - the `Path` to the file from which to read the motion.
    pub fn from_path(path: &Path) -> Result<Motion> {
        // generating the motion based on the corresponding config file
        let motion_config_data = std::fs::read_to_string(path).into_diagnostic()?;
        let config: MotionSettings = toml::de::from_str(&motion_config_data).into_diagnostic()?;

        let mut motion: Motion = Motion {
            settings: config.clone(),
            submotions: HashMap::new(),
        };

        // populating the submotions property of Motion with the correct SubMotions
        for submotion_name in &config.motion_order {
            let submotion_path = Path::new("./assets/motions/submotions")
                .join(submotion_name)
                .with_extension("json");
            if !submotion_path.exists() {
                return Err(miette! {
                    "Submotion {:?} does not exist, no file: {:?} could be found", submotion_name, submotion_path
                });
            }
            let submotion: SubMotion = serde_json::from_reader(
                File::open(submotion_path).into_diagnostic()?,
            )
            .map_err(|err| {
                miette!(format!(
                    "Could not load submotion file during construction of motion, {}",
                    err
                ))
            })?;
            motion.submotions.insert(submotion_name.clone(), submotion);
        }

        Ok(motion)
    }

    /// Helper function for editing the duration of the first movement of a motion.
    /// This can be helpful when preventing the robot from moving to the initial
    /// position with a dangerous speed.
    ///
    /// # Arguments
    /// * `submotion_name` - name of the current submotion.
    /// * `duration` - new duration for the initial movement.
    pub fn set_initial_duration(&mut self, submotion_name: &String, duration: Duration) {
        self.submotions
            .get_mut(submotion_name)
            .expect("Submotion not present")
            .keyframes[0]
            .duration = duration;
    }
}

/// Stores information about the currently active motion.
#[derive(Debug, Clone)]
pub struct ActiveMotion {
    pub motion: Motion,
    pub cur_sub_motion: (String, usize),
    pub cur_keyframe_index: usize,
    pub movement_start: Instant,
}

impl ActiveMotion {
    /// Returns the next position the robot should be in next by interpolating between the previous and next keyframe.
    /// If the current submotion has ended, will return None.
    pub fn get_position(&mut self) -> Option<JointArray<f32>> {
        let keyframes = &self.motion.submotions[&self.cur_sub_motion.0].keyframes;

        // Check if we have reached the end of the current submotion
        if keyframes.len() < self.cur_keyframe_index + 1 {
            return None;
        }

        let previous_position =
            &keyframes[self.cur_keyframe_index.saturating_sub(1)].target_position;
        let current_movement = &keyframes[self.cur_keyframe_index];

        // if the current movement has been completed:
        if self.movement_start.elapsed().as_secs_f32()
            > keyframes[self.cur_keyframe_index].duration.as_secs_f32()
        {
            // update the index
            self.cur_keyframe_index += 1;

            // Check if there exists a next keyframe
            if keyframes.len() < self.cur_keyframe_index + 1 {
                return None;
            }

            // update the starting time of the movement
            self.movement_start = Instant::now();
        }

        Some(interpolate_jointarrays(
            previous_position,
            &current_movement.target_position,
            (self.movement_start.elapsed()).as_secs_f32() / current_movement.duration.as_secs_f32(),
            &self.motion.settings.interpolation_type,
        ))
    }

    /// Fetches the next submotion name to be executed.
    #[must_use]
    pub fn get_next_submotion(&self) -> Option<&String> {
        let next_index = self.cur_sub_motion.1 + 1;
        self.motion.settings.motion_order.get(next_index)
    }

    /// Returns the first movement the robot would make for the chosen submotion.
    ///
    /// # Arguments
    /// * `submotion_name` - name of the submotion.
    #[must_use]
    pub fn initial_movement(&self, submotion_name: &String) -> &Movement {
        &self.motion.submotions[submotion_name].keyframes[0]
    }

    /// Transitions the `ActiveMotion` to the next submotion.
    pub fn transition(&mut self, submotion_name: String) {
        if let Some(new_index) = self
            .motion
            .settings
            .motion_order
            .iter()
            .position(|x| *x == submotion_name)
        {
            self.cur_sub_motion = (submotion_name, new_index);
            self.cur_keyframe_index = 0;
            self.movement_start = Instant::now();
        } else {
            error!(
                "Motion transition has failed! Could not find submotion with name: {}",
                submotion_name
            );
            return;
        }
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
    StandupBack,
    StandupStomach,
    Test,
}
