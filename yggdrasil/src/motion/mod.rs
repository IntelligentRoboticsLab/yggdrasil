use crate::{kinematics, nao, prelude::*};

use self::odometry::Odometry;

pub mod odometry;

/// The motion module provides motion related functionalities.
///
/// This module provides the following resources to the application:
/// - [`Odometry`]
pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.init_resource::<Odometry>()?.add_system_chain((
            odometry::update_odometry.after(kinematics::update_kinematics),
            odometry::log_odometry,
        )))
    }
}
