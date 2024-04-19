use crate::{filter, kinematics, nao::manager::finalize, prelude::*};

use self::odometry::Odometry;
use miette::Result;

pub mod motion_executer;
pub mod motion_manager;
pub mod motion_types;
pub mod motion_util;
pub mod odometry;
pub mod path_finding;
pub mod step_planner;

use motion_executer::motion_executer;
use motion_manager::motion_manager_initializer;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`MotionManager`](`motion_manager::MotionManager`)
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<Odometry>()?
            .add_system_chain((
                odometry::update_odometry
                    .after(kinematics::update_kinematics)
                    .after(filter::orientation::update_orientation),
                odometry::log_odometry,
            ))
            .add_startup_system(odometry::setup_viewcoordinates)?
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(finalize))
            .add_module(step_planner::StepPlannerModule))
    }
}
