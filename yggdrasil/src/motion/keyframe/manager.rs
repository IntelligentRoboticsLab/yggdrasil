use super::types::{ActiveMotion, Motion, MotionType};
use bevy::prelude::*;
use miette::Result;
use nidhogg::types::JointArray;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
#[derive(Default, Debug, Resource)]
pub struct AnimationManager {
    pub motions: HashMap<MotionType, Motion>,
    pub active_motion: Option<ActiveMotion>,
    pub source_position: Option<JointArray<f32>>,
    pub motion_execution_starting_time: Option<Instant>,
    pub submotion_execution_starting_time: Option<Instant>,
    pub submotion_finishing_time: Option<Instant>,
}

impl AnimationManager {
    /// Initializes a `AnimationManager`.
    #[must_use]
    pub fn new() -> Self {
        AnimationManager::default()
    }

    /// Simple abstraction function for checking whether a motion is currently active
    #[must_use]
    pub fn is_motion_active(&self) -> bool {
        self.active_motion.is_some()
    }

    /// Adds a motion to the `AnimationManager`.
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
