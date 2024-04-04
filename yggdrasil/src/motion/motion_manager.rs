use crate::motion::motion_types::{
    ConditionalVariable, FailRoutine, Motion, MotionCondition, MotionType,
};
use miette::Result;
use nidhogg::NaoState;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use tyr::prelude::*;

#[derive(Clone)]
pub struct ActiveMotion {
    /// Current motion.
    pub motion: Motion,
    /// name and index of current submotion being executed
    pub cur_sub_motion: (String, i32),
    /// Previous Keyframe index
    pub prev_keyframe_index: i32,
    /// Current movement starting time
    pub movement_start: Instant,
}

impl ActiveMotion {
    /// Fetches the next submotion name to be executed.
    pub fn get_next_submotion(&self) -> Option<String> {
        let next_index = self.cur_sub_motion.1 as usize + 1;

        // check whether a next submotion exists
        if self.motion.motion_settings.motion_order.len() >= next_index + 1 {
            return Some(self.motion.motion_settings.motion_order[next_index].clone());
        }

        // if no submotion exists, the motion has ended
        None
    }

    /// Returns the next submotion to be executed.
    ///
    /// # Arguments
    ///
    /// * `nao_state` - Current state of the Nao.
    /// * `submotion_name` - Name of the next submotion.
    pub fn transition(
        &mut self,
        nao_state: &mut NaoState,
        submotion_name: String,
    ) -> Option<ActiveMotion> {
        let next_submotion = self.motion.submotions[&submotion_name].clone();

        for condition in next_submotion.conditions {
            if !check_condition(nao_state, condition) {
                return select_routine(
                    self.clone(),
                    self.motion.submotions[&self.cur_sub_motion.0]
                        .fail_routine
                        .clone(),
                );
            }
        }

        println!("\n\nTransition");
        self.cur_sub_motion = (submotion_name, self.cur_sub_motion.1 + 1);
        self.prev_keyframe_index = 0;
        self.movement_start = Instant::now();

        Some(self.clone())
    }
}

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
pub struct MotionManager {
    /// Keeps track of information about the active motion.
    pub active_motion: Option<ActiveMotion>,
    /// Keeps track of when the execution of a motion started.
    pub motion_execution_starting_time: Option<Instant>,
    // Keeps track of when the execution of the current submotion started.
    pub submotion_execution_starting_time: Option<Instant>,
    // TODO
    pub submotion_finishing_time: Option<Instant>,
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
            submotion_execution_starting_time: None,
            submotion_finishing_time: None,
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
            cur_sub_motion: (chosen_motion.motion_settings.motion_order[0].clone(), 0),
            prev_keyframe_index: 0,
            motion: chosen_motion,
            movement_start: Instant::now(),
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
    motion_manager.add_motion(MotionType::Test, "./assets/motions/complex_test.toml")?;
    motion_manager.add_motion(
        MotionType::StandupFaceDown,
        "./assets/motions/StandupTest.toml",
    )?;
    motion_manager.add_motion(
        MotionType::StandupFaceDownV2,
        "./assets/motions/StandupTest_V2.toml",
    )?;
    // motion_manager.add_motion(
    //     MotionType::FallForwards,
    //     "./assets/motions/fallforwards.json",
    // )?;
    // motion_manager.add_motion(
    //     MotionType::FallBackwards,
    //     "./assets/motions/fallbackwards.json",
    // )?;
    // motion_manager.add_motion(
    //     MotionType::FallLeftways,
    //     "./assets/motions/fallleftways.json",
    // )?;
    // motion_manager.add_motion(
    //     MotionType::FallRightways,
    //     "./assets/motions/fallrightways.json",
    // )?;
    // motion_manager.add_motion(MotionType::Neutral, "./assets/motions/neutral.json")?;
    // motion_manager.add_motion(MotionType::Example, "./assets/motions/example.json")?;
    storage.add_resource(Resource::new(motion_manager))?;

    Ok(())
}

/// Checks whether the current NaoState fulfills a specified condition.
///
/// # Arguments
///
/// * `nao_state` - Current state of the Nao.
/// * `condition` - The condition which needs to be checked.
fn check_condition(nao_state: &mut NaoState, condition: MotionCondition) -> bool {
    match condition.variable {
        ConditionalVariable::GyroscopeX => {
            nao_state.gyroscope.x > condition.min && nao_state.gyroscope.x < condition.max
        }
        ConditionalVariable::GyroscopeY => {
            nao_state.gyroscope.y > condition.min && nao_state.gyroscope.y < condition.max
        }
        ConditionalVariable::AngleX => {
            nao_state.angles.x > condition.min && nao_state.angles.x < condition.max
        }
        ConditionalVariable::AngleY => {
            nao_state.angles.y > condition.min && nao_state.angles.y < condition.max
        }
    }
}

/// Matches a specified motion fail routine with the correct next motion.
///
/// # Arguments
///
/// * `active_motion` - The current active motion of the Nao.
/// * `routine` - The routine that will be matched with an according motion.
fn select_routine(mut active_motion: ActiveMotion, routine: FailRoutine) -> Option<ActiveMotion> {
    match routine {
        // aborts the current motion
        FailRoutine::Abort => None,
        // TODO implement catch routine
        FailRoutine::Catch => None,
        // retry the previous submotion
        FailRoutine::Retry => {
            active_motion.prev_keyframe_index = 0;
            active_motion.movement_start = Instant::now();
            Some(active_motion)
        }
    }
}
