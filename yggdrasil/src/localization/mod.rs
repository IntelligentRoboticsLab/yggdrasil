use crate::{
    motion::odometry::{self, Odometry},
    prelude::*,
};
use nalgebra::{Isometry2, Point2};

pub struct LocalizationModule;

impl Module for LocalizationModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(update_robot_pose.after(odometry::update_odometry))
            .add_resource(Resource::new(RobotPose::default()))
    }
}

#[derive(Default, Debug, Clone)]
pub struct RobotPose {
    pub inner: Isometry2<f32>,
}

impl RobotPose {
    /// The current position of the robot in the world, in absolute coordinates.
    ///
    /// The center of the world is at the center of the field, with the x-axis pointing towards the
    /// opponent's goal.
    pub fn world_position(&self) -> Point2<f32> {
        self.inner.translation.vector.into()
    }

    /// The current rotation of the robot in the world, in radians.
    pub fn world_rotation(&self) -> f32 {
        self.inner.rotation.angle()
    }

    /// Transform a point from robot coordinates to world coordinates.
    pub fn robot_to_world(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner * point
    }

    /// Transform a point from world coordinates to robot coordinates.
    pub fn world_to_robot(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.inverse_transform_point(point)
    }
}

#[system]
pub fn update_robot_pose(robot_pose: &mut RobotPose, odometry: &Odometry) -> Result<()> {
    robot_pose.inner = odometry.accumulated;

    Ok(())
}
