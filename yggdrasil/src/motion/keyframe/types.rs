use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::JointArray;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::collections::HashMap;
use std::fs::File;
use std::{path::Path, time::Duration};

use std::time::Instant;

use toml;

use super::{manager::ActiveMotion, util::lerp};

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

/// An enum containing the possible interpolation types for a motion.
///
/// # Notes
/// - New interpolation type implementations should be added as new variants to this enum.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InterpolationType {
    Linear,
    // TODO
    SmoothIn,
    // TODO
    SmoothOut,
}

/// An enum containing the possible variables that can be used as conditions
/// for entering a submotion for a robot.
///
/// # Notes
/// - New conditional variables should be added as new variants to this enum.
///   Furthermore, the implementation for checking this variable should be added
///   to the 'check_condition' function in 'keyframe_executor'.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ConditionalVariable {
    GyroscopeX,
    GyroscopeY,
    AngleX,
    AngleY,
}

/// An enum containing the failroutines that the robot can execute when it fails
/// to satisfy a condition for entering a submotion.
///
/// # Notes
/// - New failroutines should be added as new variants to this enum.
///   Furthermore, the implementation for executing this failroutine should be added
///   to the 'select_routine' function in 'keyframe_executor'.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FailRoutine {
    Retry,
    Abort,
    Catch,
    // Add new fail routines here
}

/// Enum containing the different exit routines the robot can execute
/// upon completion of a motion.
///
/// # Notes
/// - Currently only the "Standing" routine is present, which is used
///   to signify to the behaviour engine that the standup motion has
///   executed succesfully.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ExitRoutine {
    Standing,
    // Add new exit routines here
}

/// Stores information about a single conditional variable, keeping track
/// of the minimum and maximum value the variable is allowed to take.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionCondition {
    pub variable: ConditionalVariable,
    pub min: f32,
    pub max: f32,
}

/// Stores information about the different chosen motion settings.
///
/// # Notes
/// - Currently this struct only contains information about the
///   regular order of the submotions and the interpolation type used.
/// - New motion settings should be added here as a new property.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionSettings {
    // interpolation type used during the motion
    pub interpolation_type: InterpolationType,
    // exit routine to be executed when the motion has finished succesfully
    pub exit_routine: Option<ExitRoutine>,
    // the standard order the submotions will be executed in
    pub motion_order: Vec<String>,
    // New motion settings can be added here
}

/// Stores information about a submotion.
///
/// # Notes
/// - Currently does not use the chest angle bound variables,
///   but this will be implemented soon(tm).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubMotion {
    /// Joint stiffness of the submotion.
    pub joint_stifness: f32,
    /// TODO, upper limit for angle variable.
    pub chest_angle_bound_upper: f32,
    /// TODO, lower limit for angle variable.
    pub chest_angle_bound_lower: f32,
    /// Amount of time in seconds that the submotion will wait after finishing.
    pub exit_waittime: f32,
    /// Routine that the robot will execute if the current submotion fails.
    pub fail_routine: FailRoutine,
    /// Conditions the robot must fulfill to be able to enter the submotion.
    pub conditions: Vec<MotionCondition>,
    /// The keyframes which comprise the submotion.
    pub keyframes: Vec<Movement>,
}

/// Stores information about a motion.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Motion {
    /// Motion settings connected to the current motion.
    pub settings: MotionSettings,
    /// The different submotions contained in the motion.
    pub submotions: HashMap<String, SubMotion>,
}

impl Motion {
    /// Initializes a motion from a motion config file. Uses serde for deserialization.
    /// Generates the appropriate motion file from a motion config file if this file
    /// is not present. Otherwise, uses the existing motion file.
    ///
    /// # Arguments
    /// * `path` - the `Path` to the file from which to read the motion.
    pub fn from_path(path: &Path) -> Result<Motion> {
        // generating the motion based on the corresponding config file
        let motion_config_data = std::fs::read_to_string(path).into_diagnostic()?;
        let config: MotionSettings = toml::de::from_str(&motion_config_data).into_diagnostic()?;

        // based on the gathered config file, we now generate a new Motion
        let mut motion: Motion = Motion {
            settings: config.clone(),
            submotions: HashMap::new(),
        };

        // populating the submotions property of Motion with the correct SubMotions
        for submotion_name in config.motion_order.iter() {
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

        // when the Motion has been created, we save it to the assets/motions folder
        serde_json::to_writer(
            &File::create(path.with_extension("json")).into_diagnostic()?,
            &motion,
        )
        .into_diagnostic()?;

        Ok(motion)
    }

    /// Returns the next position the robot should be in next by interpolating between the previous and next keyframe.
    ///
    /// # Arguments
    /// * `current_sub_motion` - the current sub motion the robot is executing.
    /// * `active_motion` - the currently active motion
    pub fn get_position(
        &self,
        current_sub_motion: &String,
        active_motion: &mut ActiveMotion,
    ) -> Option<JointArray<f32>> {
        let keyframes = &self.submotions[current_sub_motion].keyframes;

        // Check if we have reached the end of the current submotion
        if keyframes.len() < active_motion.cur_keyframe_index + 1 {
            return None;
        }

        // if the current movement has been completed:
        if active_motion.movement_start.elapsed().as_secs_f32()
            > keyframes[active_motion.cur_keyframe_index]
                .duration
                .as_secs_f32()
        {
            // update the index
            active_motion.cur_keyframe_index += 1;

            // Check if there exists a next keyframe
            if keyframes.len() < active_motion.cur_keyframe_index + 1 {
                return None;
            }

            // update the time of the start of the movement
            active_motion.movement_start = Instant::now();
        }

        Some(lerp(
            &keyframes[active_motion.cur_keyframe_index - 1].target_position,
            &keyframes[active_motion.cur_keyframe_index].target_position,
            (active_motion.movement_start.elapsed()).as_secs_f32()
                / keyframes[active_motion.cur_keyframe_index]
                    .duration
                    .as_secs_f32(),
        ))
    }

    /// Returns the first movement the robot would make for the current submotion.
    ///
    /// # Arguments
    /// * `submotion_name` - name of the current submotion.
    pub fn initial_movement(&self, submotion_name: &String) -> &Movement {
        &self.submotions[submotion_name].keyframes[0]
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
