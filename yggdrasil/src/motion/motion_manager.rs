use crate::motion::motion_types::{Motion, MotionType};
use miette::Result;
use nidhogg::NaoState;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tyr::prelude::*;

use super::motion_types::{FailRoutine, MotionCondition};

#[derive(Clone)]
pub struct ActiveMotion {
    /// Current motion.
    pub motion: Motion,
    /// current submotion being executed
    pub current_sub_motion: (String, i32),
    /// Previous Keyframe
    pub prev_keyframe_index: i32,
    /// Current movement starting time
    pub movement_start: SystemTime,
    /// Keeps track of when a motion started.
    pub starting_time: SystemTime,
}

impl ActiveMotion {
    /// Fetches the next submotion name to be executed.
    pub fn get_next_submotion(&self) -> Option<String> {
        let next_index = self.current_sub_motion.1 as usize + 1;

        // check whether a next submotion exists
        if self.motion.motion_settings.motion_order.len() >= next_index + 1 {
            return Some(self.motion.motion_settings.motion_order[next_index].clone());
        }

        None
    }

    pub fn transition(&self, nao_state: &mut NaoState, submotion_name: String) -> ActiveMotion {
        let next_submotion = self.motion.submotions[&submotion_name];

        for condition in next_submotion.conditions {
            if !check_condition(nao_state, condition) {
                return select_routine(
                    self.motion.submotions[&self.current_sub_motion.0].fail_routine,
                );
            }
        }

        self.current_sub_motion = (submotion_name, self.current_sub_motion.1 + 1);
        self.prev_keyframe_index = 0;
        self.movement_start = SystemTime::now();

        *self
    }
}

pub fn check_condition(nao_state: &mut NaoState, condition: MotionCondition) -> bool {
    // TODO
    true
}

pub fn select_routine(routine: FailRoutine) -> ActiveMotion {
    // TODO
}

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
pub struct MotionManager {
    /// Keeps track of information about the active motion.
    pub active_motion: Option<ActiveMotion>,
    /// Keeps track of when the execution of a motion started.
    pub motion_execution_starting_time: Option<SystemTime>,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    pub motions: HashMap<MotionType, Motion>,
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
    pub fn add_motion(&mut self, motion_type: MotionType, motion_file: &'static str) -> Result<()> {
        self.motions
            .insert(motion_type, Motion::from_path(Path::new(motion_file))?);
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

        self.motion_execution_starting_time = None;

        let chosen_motion = self
            .motions
            .get(&motion_type)
            .cloned()
            .expect("Motion type not added to the motion manager");

        self.active_motion = Some(ActiveMotion {
            current_sub_motion: (chosen_motion.motion_settings.motion_order[0].clone(), 0),
            prev_keyframe_index: 0,
            motion: chosen_motion,
            movement_start: SystemTime::now(),
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
        MotionType::FallForwards,
        "./assets/motions/fallforwards.json",
    )?;
    motion_manager.add_motion(
        MotionType::FallBackwards,
        "./assets/motions/fallbackwards.json",
    )?;
    motion_manager.add_motion(
        MotionType::FallLeftways,
        "./assets/motions/fallleftways.json",
    )?;
    motion_manager.add_motion(
        MotionType::FallRightways,
        "./assets/motions/fallrightways.json",
    )?;
    motion_manager.add_motion(MotionType::Neutral, "./assets/motions/neutral.json")?;
    motion_manager.add_motion(MotionType::Example, "./assets/motions/example.json")?;
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}
