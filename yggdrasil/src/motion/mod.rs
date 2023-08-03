use crate::nao::write_hardware_info;
use miette::Result;
use tyr::prelude::*;

mod motion_executer;
mod motion_manager;
mod motion_types;

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
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(write_hardware_info)))
    }
}
