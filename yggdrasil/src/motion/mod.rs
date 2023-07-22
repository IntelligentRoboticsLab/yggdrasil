use color_eyre::Result;
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

/// TODO: Implement iterator for JointArray for interpolation

#[serde_as]
#[derive(Deserialize, Debug)]
/// Represents a single robot movement.
pub struct Movement {
    /// Movement target joint positions.
    target_positions: JointArray<f32>,
    /// Movement duration.
    #[serde_as(as = "DurationSecondsWithFrac<f64>")]
    duration: Duration,
}

#[derive(Deserialize, Debug)]
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
    pub fn from_path(path: &Path) -> Motion {
        let file = File::open(&path)
            .expect(format!("Could not read motion file {}.", path.display()).as_str());
        serde_json::from_reader(file)
            .expect(format!("Could deserialize json {}.", path.display()).as_str())
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
/// An enumeration of all possible motions.
pub enum MotionType {
    SitDown,
    StandUp,
}

/// Manages motions, stores all possible motions and keeps track of information
/// about the motion that is currently being executed.
struct MotionManager {
    /// `MotionType` of the current motion.
    current_motion: Option<MotionType>,
    /// Keeps track of when a motion started.
    current_motion_starting_time: Option<SystemTime>,
    /// Needed for checking if a motion can start.
    in_current_motion_initial_position: Option<bool>,
    /// Contains the mapping from `MotionTypes` to `Motion`.
    motions: HashMap<MotionType, Motion>,
}

impl MotionManager {
    /// Initializes a `MotionManger`.
    ///
    /// # Arguments
    ///
    /// * `motions` -  A mapping from motion types to the files where the
    ///                motions are stored.
    pub fn new(motions: HashMap<MotionType, &str>) -> Self {
        MotionManager {
            current_motion: None,
            current_motion_starting_time: None,
            in_current_motion_initial_position: None,
            motions: motions
                .into_iter()
                .map(|(k, v)| (k, Motion::from_path(Path::new(v))))
                .collect(),
        }
    }

    /// Starts a new motion.
    ///
    /// # Arguments
    ///
    /// * `motion_type` - the motion to start.
    pub fn start_new_motion(&mut self, motion_type: MotionType) {
        self.current_motion_starting_time = Some(std::time::SystemTime::now());
        self.current_motion = Some(motion_type);

        // TODO: check this, lerp to starting position if not there yet.
        self.in_current_motion_initial_position = Some(true)
    }
}

#[system]
fn motion_executer(nao_state: &mut NaoState, motion_manager: &mut MotionManager) -> Result<()> {
    nao_state.position = JointArray::<f32>::default();
    nao_state.stiffness = JointArray::<f32>::default();

    if let Some(motion_type) = &motion_manager.current_motion {
        println!("test {:?}", motion_type);
    }

    Ok(())
}

pub struct MotionModule;
impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        let motion_manager = MotionManager::new(HashMap::from([
            (MotionType::SitDown, "./sit_down_motion.json"),
            (MotionType::StandUp, "./stand_up_motion.json"),
        ]));

        app.add_system(motion_executer)
            .add_resource(Resource::new(motion_manager))
    }
}
