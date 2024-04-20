use crate::{filter, kinematics, prelude::*};

use self::odometry::Odometry;
use crate::nao::write_hardware_info;
use miette::Result;

pub mod motion_executer;
pub mod motion_manager;
pub mod motion_types;
pub mod motion_util;
pub mod odometry;

use motion_executer::motion_executer;
use motion_manager::motion_manager_initializer;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`MotionManager`](`motion_manager::MotionManager`)
/// - [`Odometry`]
pub struct MotionModule;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct MotionConfig {
    pub maximum_joint_speed: f32,
    pub max_stable_gyro_value: f32,
    pub max_stable_acc_value: f32,
    pub mix_stable_fsr_value: f32,
    pub minimum_wait_time: f32,
}

impl Config for MotionConfig {
    const PATH: &'static str = "motion.toml";
}

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
            .init_config::<MotionConfig>()?
            .add_startup_system(motion_manager_initializer)?
            .add_system(motion_executer.after(write_hardware_info)))
    }
}
