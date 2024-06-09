use crate::{kinematics, nao::manager::finalize, prelude::*, sensor};

use self::odometry::Odometry;
use miette::Result;

pub mod keyframe;
pub mod odometry;
pub mod path_finding;
pub mod step_planner;
pub mod walk;

use keyframe::executor::motion_executer;
use keyframe::manager::motion_manager_initializer;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`MotionManager`](`motion_manager::MotionManager`)
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.init_resource::<Odometry>()?
            .add_system(
                odometry::update_odometry
                    .after(kinematics::update_kinematics)
                    .after(sensor::orientation::update_orientation),
            )
            .add_startup_system(odometry::setup_viewcoordinates)?
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(finalize))
            .add_module(step_planner::StepPlannerModule)
    }
}
