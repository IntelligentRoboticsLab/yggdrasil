use crate::nao::write_hardware_info;
use miette::Result;
use tyr::prelude::*;

pub mod motion_executer;
pub mod motion_manager;
<<<<<<< HEAD
pub mod motion_types;
pub mod motion_util;

=======
pub mod motion_recorder;
pub mod motion_types;
pub mod motion_util;

use self::motion_recorder::Test;
>>>>>>> ceeff45ea380ffba4d81a2169e6c3717906344fd
use motion_executer::motion_executer;
use motion_manager::motion_manager_initializer;

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
<<<<<<< HEAD
=======
            .add_module(Test)?
>>>>>>> ceeff45ea380ffba4d81a2169e6c3717906344fd
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(write_hardware_info)))
    }
}
