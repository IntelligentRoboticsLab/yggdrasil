use color_eyre::{eyre::eyre, Result};
use nidhogg::types::JointArray;
use nidhogg::NaoState;
use serde::Deserialize;
use serde_with::{serde_as, DurationSecondsWithFrac};
use std::collections::HashMap;
use std::fs::File;
use std::time::SystemTime;
use std::{path::Path, time::Duration};
use tyr::prelude::*;
use tyr::Module;

const STARTING_POSITION_ERROR_MARGIN: f32 = 0.05;
const LERP_TO_STARTING_POSITION_DURATION_SECS: f32 = 5.0;

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
        match serde_json::from_reader(File::open(path)?) {
            Ok(val) => Ok(val),
            Err(err) => Err(eyre!("Could deserialize json {}: {}.", path.display(), err)),
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

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
struct MotionManager {
    /// Current motion.
    current_motion: Option<Motion>,
    /// Keeps track of when a motion started.
    motion_starting_time: Option<SystemTime>,
    /// Keeps track of when the execution of a motion started.
    motion_execution_starting_time: Option<SystemTime>,
    /// Needed for checking if initial position still needs to be reached.
    started_executing_motion: bool,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    motions: HashMap<MotionType, Motion>,
}

impl MotionManager {
    /// Initializes a `MotionManger`.
    ///
    /// # Arguments
    ///
    /// * `motions` - A mapping from motion types to the files where the
    ///               motions are stored.
    pub fn new() -> Self {
        MotionManager {
            current_motion: None,
            motion_starting_time: None,
            motion_execution_starting_time: None,
            started_executing_motion: false,
            motions: HashMap::new(),
        }
    }

    /// Adds a motion to the `MotionManger`.
    ///
    /// # Arguments
    ///
    /// * `motion_type` - Type of the motion.
    /// * `motion_file` - Path to the file where the motion movements can be found.
    pub fn add_motion(mut self, motion_type: MotionType, motion_file: &str) -> Result<Self> {
        self.motions
            .insert(motion_type, Motion::from_path(Path::new(motion_file))?);
        Ok(self)
    }

    /// Starts a new motion.
    ///
    /// # Arguments
    ///
    /// * `motion_type` - The type of motion to start.
    pub fn start_new_motion(&mut self, motion_type: MotionType) {
        self.started_executing_motion = false;
        self.current_motion = self.motions.get(&motion_type).cloned();
    }
}

/// Checks if the current position has reached the target position with a certain
/// margin of error.
///
/// # Arguments
///
/// * `current_positions` - Positions of which you want to check if they have reached a certain
///                         position.
/// * `target_positions` - Positions of which you want to check if they ahve been reached.
/// * `error_margin` - Range within which a target position has been reached.
fn reached_position(
    current_positions: &JointArray<f32>,
    target_positions: &JointArray<f32>,
    error_margin: f32,
) -> bool {
    let curr_iter = current_positions.clone().into_iter();
    let target_iter = target_positions.clone().into_iter();

    curr_iter
        .zip(target_iter)
        .map(|(curr, target)| target - error_margin <= curr && curr <= target + error_margin)
        .collect::<Vec<bool>>()
        .contains(&false)
}

/// Performs linear interpolation between two `JointArray<f32>`.
///
/// # Arguments
///
/// * `current_positions` - Starting position.
/// * `target_positions` - Final position.
/// * `scalar` - Scalar from 0-1 that indicates what weight to assign to each position.
fn lerp(
    current_positions: &JointArray<f32>,
    target_positions: &JointArray<f32>,
    scalar: f32,
) -> JointArray<f32> {
    let curr_iter = current_positions.clone().into_iter();
    let target_iter = target_positions.clone().into_iter();

    curr_iter
        .zip(target_iter)
        .map(|(curr, target)| curr * scalar + target * (1.0 - scalar))
        .collect()
}

/// TODO: docs + implementation
/// Retrieves the current position by using linear interpolation between the
/// two nearest positions based on the starting time and current time.
///
/// # Arguments
///
/// * `motion` - Current `Motion`.
fn get_positions(motion: &Motion, duration: &Duration) -> Option<JointArray<f32>> {
    motion
        .get_surrounding_frames(duration)
        .map(|(frame_a, frame_b)| {
            lerp(
                &frame_a.target_positions,
                &frame_b.target_positions,
                duration.as_secs_f32() / LERP_TO_STARTING_POSITION_DURATION_SECS,
            )
        })
}

/// Executes the current motion.
///
/// # Arguments
///
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
#[system]
fn motion_executer(nao_state: &mut NaoState, motion_manager: &mut MotionManager) -> Result<()> {
    if let Some(motion) = motion_manager.current_motion.clone() {
        if !motion_manager.started_executing_motion {
            if motion_manager.motion_starting_time.is_none() {
                motion_manager.motion_starting_time = Some(SystemTime::now());
            }

            if !reached_position(
                &nao_state.position,
                &motion.initial_positions,
                STARTING_POSITION_ERROR_MARGIN,
            ) {
                // Starting position has not yet been reached, so lerp to start position, untill
                // position has been reached.
                let elapsed_time_since_start_of_motion: f32 = motion_manager
                    .motion_starting_time
                    .unwrap()
                    .elapsed()
                    .unwrap()
                    .as_secs_f32();

                nao_state.position = lerp(
                    &nao_state.position,
                    &motion.initial_positions,
                    elapsed_time_since_start_of_motion / LERP_TO_STARTING_POSITION_DURATION_SECS,
                );

                return Ok(());
            } else {
                motion_manager.motion_execution_starting_time = Some(SystemTime::now());
            }

            match get_positions(
                &motion,
                &motion_manager
                    .motion_execution_starting_time
                    .unwrap()
                    .elapsed()
                    .unwrap(),
            ) {
                Some(position) => {
                    nao_state.position = position;
                    // TODO: Add this to the motion files.
                    nao_state.stiffness = JointArray::<f32>::default();
                }
                None => {
                    //Current motion is finished.
                    motion_manager.current_motion = None;
                    motion_manager.motion_starting_time = None;
                    motion_manager.motion_execution_starting_time = None;
                    motion_manager.started_executing_motion = false;
                }
            }
        }
    };

    Ok(())
}

/// Initializes the `MotionManager`. Adds motions to the `MotionManger` by reading
/// and deserializing the motions from motion files. Then adds the `MotionManager`
/// as resource
///
/// # Arguments
///
/// * `storage` - System storage.
fn motion_manager_initializer(storage: &mut Storage) -> Result<()> {
    let mut motion_manager = MotionManager::new()
        .add_motion(
            MotionType::SitDownFromStand,
            "./sit_down_from_stand_motion.json",
        )?
        .add_motion(
            MotionType::StandUpFromSit,
            "./stand_up_from_sit_motion.json",
        )?;

    // TODO: remove this, this is for testing
    motion_manager.start_new_motion(MotionType::StandUpFromSit);
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}

/// Module used to add all necessary resource for playing motions to the system.
pub struct MotionModule;

impl Module for MotionModule {
    /// Initializes the `MotionModule`. By adding the `motion_manager_initializer`
    /// and the `motion_executor` to the system.
    ///
    /// # Arguments
    ///
    /// * `app` - App.
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer))
    }
}
