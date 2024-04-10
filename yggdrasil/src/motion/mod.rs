use self::odometry::Odometry;
use crate::kinematics;
use crate::nao::write_hardware_info;
use miette::Result;
use tyr::prelude::*;

pub mod motion_executer;
pub mod motion_manager;
pub mod motion_tester;
pub mod motion_types;
pub mod motion_util;
pub mod odometry;

use self::motion_tester::MotionTester;
use motion_executer::motion_executer;
use motion_manager::motion_manager_initializer;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<Odometry>()?
            .add_system_chain((
                odometry::update_odometry.after(kinematics::update_kinematics),
                odometry::log_odometry,
            ))
            .add_module(MotionTester)?
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(write_hardware_info)))
    }
}
