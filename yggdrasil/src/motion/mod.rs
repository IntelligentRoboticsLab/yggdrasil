use crate::{kinematics, prelude::*, sensor};

use self::odometry::Odometry;
use miette::Result;

pub mod keyframe;
pub mod odometry;
pub mod path_finding;
pub mod step_planner;
pub mod walk;

use keyframe::executor::keyframe_executor;
use keyframe::manager::keyframe_executor_initializer;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`KeyframeExecutor`](`keyframe::KeyframeExecutor`)
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.init_resource::<Odometry>()?
            .add_staged_system(
                SystemStage::Sensor,
                odometry::update_odometry
                    .after(kinematics::update_kinematics)
                    .after(sensor::orientation::update_orientation),
            )
            .add_startup_system(odometry::setup_viewcoordinates)?
            .add_startup_system(keyframe_executor_initializer)?
            .add_staged_system(SystemStage::PostWrite, keyframe_executor)
            .add_module(step_planner::StepPlannerModule)
    }
}
