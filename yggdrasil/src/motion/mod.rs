use crate::{filter::button::HeadButtons, nao::write_hardware_info};
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage,
};
use tyr::prelude::*;

mod motion_executer;
mod motion_manager;
mod motion_types;
mod motion_util;

use motion_executer::motion_executer;
use motion_manager::motion_manager_initializer;

use self::motion_manager::MotionManager;

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
            .add_system(motion_executer.after(write_hardware_info))
            .add_system(testing))
    }
}

#[system]
fn testing(
    head_button: &HeadButtons,
    motion_manager: &mut MotionManager,
    nao_control_message: &mut NaoControlMessage,
) -> Result<()> {
    if head_button.middle.is_pressed() {
        // Relax all joints.
        motion_manager.stop_motion();
        nao_control_message.stiffness = JointArray::<f32>::fill(0.0);
    }

    if head_button.front.is_pressed() {
        // Play example motion.
        motion_manager.start_new_motion(motion_types::MotionType::Example);
    }

    Ok(())
}
