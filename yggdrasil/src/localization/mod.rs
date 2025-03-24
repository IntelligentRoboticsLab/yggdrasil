pub mod correction;
pub mod correspondence;
pub mod pose;

pub use pose::RobotPose;

use bevy::prelude::*;

use crate::{
    behavior::{behaviors::Standup, engine::in_behavior},
    motion::odometry,
};
use pose::{init_pose, setup_pose_visualization, update_robot_pose, visualize_pose};

/// The localization plugin provides functionalities related to the localization of the robot.
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, (init_pose, setup_pose_visualization))
            .add_systems(
                PreUpdate,
                update_robot_pose
                    .after(odometry::update_odometry)
                    .run_if(not(in_behavior::<Standup>)),
            )
            .add_systems(PostUpdate, visualize_pose);
    }
}
