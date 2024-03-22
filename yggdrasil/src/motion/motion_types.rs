use crate::motion::motion_util::lerp;
use miette::{miette, IntoDiagnostic, Result};
use nidhogg::types::{FillExt, JointArray};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::collections::HashMap;
use std::fs::File;
use std::{path::Path, time::Duration};

use std::time::Instant;

use toml;

use super::motion_manager::ActiveMotion;

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
pub enum FailRoutine {
    Retry,
    Abort,
    Catch,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionCondition {
    pub variable: ConditionalVariable,
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MotionSettings {
    pub interpolation_type: InterpolationType,
    pub wait_time: f32,
    pub motion_order: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SubMotion {
    pub joint_stifness: f32,
    pub chest_angle_bound_upper: f32,
    pub chest_angle_bound_lower: f32,
    pub fail_routine: FailRoutine,
    pub conditions: Vec<MotionCondition>,
    pub keyframes: Vec<Movement>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Motion {
    pub motion_settings: MotionSettings,
    pub submotions: HashMap<String, SubMotion>,
}

impl Motion {
    /// Initializes a motion from a motion file. Uses serde for deserialization.
    ///
    /// # Arguments
    ///
    /// * `path` - the `Path` to the file from which to read the motion.
    pub fn from_path(path: &Path) -> Result<Motion> {
        let motion_path = path.with_extension("json");

        // checking whether the specified complex motion file has been generated
        // if !motion_path.exists() {
        if true {
            // if not, we generate it based on the existing config file
            let motion_config_data = std::fs::read_to_string(path).into_diagnostic()?;
            let config: MotionSettings =
                toml::de::from_str(&motion_config_data).into_diagnostic()?;

            // based on the gathered config file, we not generate a new Motion
            let mut complexmotion: Motion = Motion {
                motion_settings: config.clone(),
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
                let submotion: SubMotion =
                    serde_json::from_reader(File::open(submotion_path).into_diagnostic()?)
                        .expect("Reading Submotion file during Motion construction");
                complexmotion
                    .submotions
                    .insert(submotion_name.clone(), submotion);
            }

            // when the Motion has been created, we save it to the assets/motions folder
            serde_json::to_writer(
                &File::create(motion_path).into_diagnostic()?,
                &complexmotion,
            )
            .into_diagnostic()?;

            return Ok(complexmotion);
        } else {
            // if the json file for the Motion does exist, simply deserialize and return it
            match serde_json::from_reader(File::open(motion_path).into_diagnostic()?) {
                Ok(val) => Ok(val),
                Err(err) => Err(miette! {
                   "Could not deserialize json {}: {}", path.display(), err
                }),
            }
        }
    }

    /// Returns the next position the robot should be in by interpolating between the previous and next keyframe.
    ///
    /// # Arguments
    ///
    /// * `current_sub_motion` - the current sub motion the robot is executing.
    /// * `active_motion` - the currently active motion
    pub fn get_position(
        &self,
        current_sub_motion: &String,
        active_motion: &mut ActiveMotion,
    ) -> Option<JointArray<f32>> {
        let keyframes = &self.submotions[current_sub_motion].keyframes;

        // Check if we have reached the end of the current submotion
        if keyframes.len() < active_motion.prev_keyframe_index as usize + 2 {
            return None;
        }

        // if the current movement has been completed:
        if active_motion.movement_start.elapsed().as_secs_f32()
            > keyframes[active_motion.prev_keyframe_index as usize + 1]
                .duration
                .as_secs_f32()
        {
            // update the index
            active_motion.prev_keyframe_index += 1;

            // update the time of the start of the movement
            active_motion.movement_start = Instant::now();
        }

        let zero = &JointArray::<f32>::fill(0.0);
        let one = &JointArray::<f32>::fill(1.0);

        println!(
            "movement_start.elapsed(): {:?}",
            active_motion.movement_start.elapsed().as_secs_f32()
        );
        println!(
            "keyframes.duration: {:?}",
            keyframes[active_motion.prev_keyframe_index as usize + 1]
                .duration
                .as_secs_f32()
        );
        println!("keyframe index: {:?}", active_motion.prev_keyframe_index);
        println!("movement_start: {:?}", active_motion.movement_start);
        println!(
            "next position: {:?}\n\n",
            Some(
                lerp(
                    zero,
                    one,
                    (active_motion.movement_start.elapsed()).as_secs_f32()
                        / keyframes[active_motion.prev_keyframe_index as usize + 1]
                            .duration
                            .as_secs_f32(),
                )
                .right_ankle_pitch
            )
        );

        return Some(lerp(
            &keyframes[active_motion.prev_keyframe_index as usize].target_position,
            &keyframes[active_motion.prev_keyframe_index as usize + 1].target_position,
            (active_motion.movement_start.elapsed()).as_secs_f32()
                / keyframes[active_motion.prev_keyframe_index as usize + 1]
                    .duration
                    .as_secs_f32(),
        ));
    }

    pub fn initial_movement(&self, submotion_name: &String) -> &Movement {
        return &self.submotions[submotion_name].keyframes[0];
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
    Test,
}
