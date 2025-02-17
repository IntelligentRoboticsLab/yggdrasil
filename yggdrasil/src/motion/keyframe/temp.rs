use super::{
    types::{Motion, Movement},
    util::interpolate_jointarrays,
};
use miette::Result;
use nidhogg::types::JointArray;
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

            // update the time of the start of the movement
            self.movement_start = Instant::now();
        }

        // using the global interpolation type, unless the movement is assigned one already
        let interpolation_type = &self.motion.settings.interpolation_type;

        Some(interpolate_jointarrays(
            previous_position,
            &current_movement.target_position,
            (self.movement_start.elapsed()).as_secs_f32() / current_movement.duration.as_secs_f32(),
            interpolation_type,
        ))
    }

    /// Returns the first movement the robot would make for the current submotion.
    ///
    /// # Arguments
    /// * `submotion_name` - name of the current submotion.
    #[must_use]
    pub fn initial_movement(&self, submotion_name: &String) -> &Movement {
        &self.motion.submotions[submotion_name].keyframes[0]
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
