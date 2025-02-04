use super::types::{Motion, MotionType};
use bevy::prelude::*;
use miette::Result;
use nidhogg::types::JointArray;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Stores information about the currently active motion.
#[derive(Debug, Clone)]
pub struct ActiveMotion {
    /// Current motion.
    pub motion: Motion,
    /// name and index of current submotion being executed
    pub cur_sub_motion: (String, usize),
    /// Current Keyframe index
    pub cur_keyframe_index: usize,
    /// Current movement starting time
    pub movement_start: Instant,
}

impl ActiveMotion {
    /// Fetches the next submotion name to be executed.
    #[must_use]
    pub fn get_next_submotion(&self) -> Option<&String> {
        let next_index = self.cur_sub_motion.1 + 1;
        self.motion.settings.motion_order.get(next_index)
    }

    /// Returns the next submotion to be executed
    ///
    /// # Arguments
    /// * `submotion_name` - Name of the next submotion.
    pub fn transition(&mut self, submotion_name: String) -> Result<Option<ActiveMotion>> {
        self.cur_sub_motion = (submotion_name, self.cur_sub_motion.1 + 1);
        self.cur_keyframe_index = 0;
        self.movement_start = Instant::now();

        Ok(Some(self.clone()))
    }
}

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
#[derive(Default, Debug, Resource)]
pub struct KeyframeExecutor {
    /// Stores the currently active motion.
    pub active_motion: Option<ActiveMotion>,
    /// Keeps track of when the execution of a motion started.
    pub motion_execution_starting_time: Option<Instant>,
    // Keeps track of when the execution of the current submotion started.
    pub submotion_execution_starting_time: Option<Instant>,
    /// Keeps track of when the current submotion has finished
    pub submotion_finishing_time: Option<Instant>,
    // Keeps track of the source position from which the robot began executing a motion.
    pub source_position: Option<JointArray<f32>>,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    pub motions: HashMap<MotionType, Motion>,
}

impl KeyframeExecutor {
    /// Initializes a `KeyframeExecutor`.
    #[must_use]
    pub fn new() -> Self {
        KeyframeExecutor::default()
    }

    /// Simple abstraction function for checking whether a motion is currently active
    #[must_use]
    pub fn is_motion_active(&self) -> bool {
        self.active_motion.is_some()
    }

    /// Adds a motion to the `KeyframeExecutor`.
    ///
    /// # Arguments
    /// * `motion_type` - Type of the motion.
    /// * `motion_file` - Path to the file where the motion movements can be found.
    pub fn add_motion(&mut self, motion_type: MotionType, motion_file: &'static str) -> Result<()> {
        self.motions
            .insert(motion_type, Motion::from_path(Path::new(motion_file))?);
        Ok(())
    }

    /// Helper function for easily stopping the currently active motion.
    pub fn stop_motion(&mut self) {
        self.active_motion = None;
        self.motion_execution_starting_time = None;
        self.submotion_execution_starting_time = None;
        self.submotion_finishing_time = None;
        self.source_position = None;
    }

    /// Starts a new motion if currently no motion is being executed.
    /// Otherwise, it will stop the current motion based on `override_motion`.
    ///
    /// # Arguments
    /// * `motion_type` - The type of motion to start.
    /// * `override_motion` - Whether or not to override the current motion (If one is active).
    pub fn start_new_motion(&mut self, motion_type: MotionType, override_motion: bool) {
        // only stop the motion if override is on
        if !override_motion {
            return;
        }
        self.stop_motion();

        self.motion_execution_starting_time = None;

        let chosen_motion = self
            .motions
            .get(&motion_type)
            .cloned()
            .expect("Motion type not added to the keyframe executor");

        self.active_motion = Some(ActiveMotion {
            cur_sub_motion: (chosen_motion.settings.motion_order[0].clone(), 0),
            cur_keyframe_index: 0,
            motion: chosen_motion,
            movement_start: Instant::now(),
        });
    }
}
