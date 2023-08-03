use crate::motion::motion_types::{Motion, MotionType};
use miette::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tyr::prelude::*;

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
pub struct MotionManager {
    /// Current motion.
    pub current_motion: Option<Motion>,
    /// Keeps track of when a motion started.
    pub motion_starting_time: Option<SystemTime>,
    /// Keeps track of when the execution of a motion started.
    pub motion_execution_starting_time: Option<SystemTime>,
    /// Needed for checking if initial position still needs to be reached.
    pub started_executing_motion: bool,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    pub motions: HashMap<MotionType, Motion>,
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
    pub fn add_motion(&mut self, motion_type: MotionType, motion_file: &str) -> Result<()> {
        self.motions
            .insert(motion_type, Motion::from_path(Path::new(motion_file))?);
        Ok(())
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

/// Initializes the `MotionManager`. Adds motions to the `MotionManger` by reading
/// and deserializing the motions from motion files. Then adds the `MotionManager`
/// as resource
///
/// # Arguments
///
/// * `storage` - System storage.
pub fn motion_manager_initializer(storage: &mut Storage) -> Result<()> {
    let mut motion_manager = MotionManager::new();
    motion_manager.add_motion(
        MotionType::SitDownFromStand,
        "./sit_down_from_stand_motion.json",
    )?;
    motion_manager.add_motion(
        MotionType::StandUpFromSit,
        "./stand_up_from_sit_motion.json",
    )?;

    // TODO: remove this, this is for testing
    motion_manager.start_new_motion(MotionType::StandUpFromSit);
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}
