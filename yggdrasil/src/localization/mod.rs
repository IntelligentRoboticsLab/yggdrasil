pub mod correspondence;
pub mod pose_filter;

use bevy::prelude::*;
use correspondence::LineCorrespondencePlugin;
use heimdall::Top;
use pose_filter::PoseFilterPlugin;

pub use pose_filter::RobotPose;

/// The localization plugin provides functionalities related to the localization of the robot.
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(
            (
                LineCorrespondencePlugin::<Top>::default(),
                // Disabled for now as we kept seeing lines inside of the robot
                // LineCorrespondencePlugin::<Bottom>::default(),
                PoseFilterPlugin,
            )
                .add_systems(PostStartup, (init_pose, setup_pose_visualization))
                .add_systems(
                    PreUpdate,
                    update_robot_pose.after(odometry::update_odometry),
                )
                .add_systems(PostUpdate, visualize_pose),
        );
    }
}
