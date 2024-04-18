use crate::{filter, kinematics, prelude::*};

use self::odometry::Odometry;

pub mod odometry;
pub mod path_finding;
pub mod step_planner;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.init_resource::<Odometry>()?
            .add_system_chain((
                odometry::update_odometry
                    .after(kinematics::update_kinematics)
                    .after(filter::orientation::update_orientation),
                odometry::log_odometry,
            ))
            .add_startup_system(odometry::setup_viewcoordinates)?
            .add_module(step_planner::StepPlannerModule)
    }
}
