use crate::motion::motion_types::{Motion, MotionType};
use miette::Result;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tyr::prelude::*;

// using an enum currently to be able to have both the complexmotion and normal motion as options for activemotion
#[derive(Clone)]
pub enum MotionCategory {
    Normal,
    Complex,
}

#[derive(Clone)]
pub struct ActiveMotion {
    /// Current motion.
    pub motion: Motion,
    /// Category of current motion
    pub motioncategory: MotionCategory,
    /// Keeps track of when a motion started.
    pub starting_time: SystemTime,
}

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
pub struct MotionManager {
    /// Keeps track of information about the active motion.
    pub active_motion: Option<ActiveMotion>,
    /// Keeps track of when the execution of a motion started.
    pub motion_execution_starting_time: Option<SystemTime>,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    pub motions: HashMap<MotionType, (MotionCategory, Motion)>,
}

impl Default for MotionManager {
    fn default() -> Self {
        Self::new()
    }
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
            active_motion: None,
            motion_execution_starting_time: None,
            motions: HashMap::new(),
        }
    }

    /// Adds a motion to the `MotionManger`.
    ///
    /// # Arguments
    ///
    /// * `motion_type` - Type of the motion.
    /// * `motion_file` - Path to the file where the motion movements can be found.
    pub fn add_motion(
        &mut self,
        motion_category: MotionCategory,
        motion_type: MotionType,
        motion_file: &'static str,
    ) -> Result<()> {
        self.motions.insert(
            motion_type,
            (motion_category, Motion::from_path(Path::new(motion_file))?),
        );
        Ok(())
    }

    pub fn stop_motion(&mut self) {
        self.active_motion = None;
        self.motion_execution_starting_time = None;
    }

    /// Starts a new motion.
    ///
    /// # Arguments
    ///
    /// * `motion_type` - The type of motion to start.
    pub fn start_new_motion(&mut self, motion_type: MotionType) {
        if self.active_motion.is_some() {
            return;
        }

        // TODO will add an aditional variable to the motion types, to indicate whether normal or complex motion
        // Currently no complex motions will be detected here, so crash will ensue
        self.motion_execution_starting_time = None;

        let (chosen_motioncategory, chosen_motion) = self
            .motions
            .get(&motion_type)
            .cloned()
            .expect("Motion type not added to the motion manager");

        self.active_motion = Some(ActiveMotion {
            motion: chosen_motion,
            motioncategory: chosen_motioncategory,
            starting_time: SystemTime::now(),
        });
    }

    /// Returns the current motion.
    pub fn get_active_motion(&mut self) -> Option<ActiveMotion> {
        self.active_motion.clone()
    }
}

/// Initializes the `MotionManager`. Adds motions to the `MotionManger` by reading
/// and deserializing the motions from motion files. Then adds the `MotionManager`
/// as resource. If you want to add new motions, add the motions here.
///
/// # Arguments
///
/// * `storage` - System storage.
pub fn motion_manager_initializer(storage: &mut Storage) -> Result<()> {
    let mut motion_manager = MotionManager::new();
    // Add new motions here!
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::FallForwards,
        "./assets/motions/fallforwards.json",
    )?;
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::FallBackwards,
        "./assets/motions/fallbackwards.json",
    )?;
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::FallLeftways,
        "./assets/motions/fallleftways.json",
    )?;
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::FallRightways,
        "./assets/motions/fallrightways.json",
    )?;
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::Neutral,
        "./assets/motions/neutral.json",
    )?;
    motion_manager.add_motion(
        MotionCategory::Normal,
        MotionType::Example,
        "./assets/motions/example.json",
    )?;
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}
