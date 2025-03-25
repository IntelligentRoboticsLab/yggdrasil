pub mod correction;
pub mod correspondence;
pub mod hypothesis;
pub mod pose;

use bevy::prelude::*;

use filter::CovarianceMatrix;
use hypothesis::{filter_hypotheses, line_update, odometry_update, RobotPoseHypothesis};
use nalgebra::vector;
use pose::initial_pose;
pub use pose::RobotPose;

use rerun::{external::glam::Quat, TimeColumn};

use crate::{
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig},
        debug::DebugContext,
    },
    motion::odometry,
    nao::Cycle,
    sensor::orientation::RobotOrientation,
};

/// The localization plugin provides functionalities related to the localization of the robot.
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, (initialize_pose, setup_pose_visualization))
            .add_systems(
                PreUpdate,
                (odometry_update, line_update, filter_hypotheses)
                    .chain()
                    .after(odometry::update_odometry),
            )
            .add_systems(PostUpdate, visualize_pose);
    }
}

fn initialize_pose(mut commands: Commands, layout: Res<LayoutConfig>, player: Res<PlayerConfig>) {
    let pose = initial_pose(&layout, player.player_number);

    // TODO: config
    let initial_covariance = CovarianceMatrix::from_diagonal(&vector![0.01, 0.01, 0.01]);
    let initial_score = 10.0;

    let hypothesis = RobotPoseHypothesis::new(pose, initial_covariance, initial_score);

    commands.spawn(hypothesis);
    commands.insert_resource(pose);
}

fn setup_pose_visualization(dbg: DebugContext) {
    let times = TimeColumn::new_sequence("cycle", [0]);
    let color_and_shape = rerun::Boxes3D::update_fields()
        .with_half_sizes([(0.075, 0.1375, 0.2865)])
        .with_colors([rerun::Color::from_rgb(0, 120, 255)])
        .columns_of_unit_batches()
        .expect("failed to create pose visualization");

    let transform = rerun::Transform3D::update_fields()
        .with_axis_length(0.3)
        .columns_of_unit_batches()
        .expect("failed to create view coordinates for pose visualation");

    dbg.send_columns(
        "localization/pose",
        [times],
        color_and_shape.chain(transform),
    );
}

fn visualize_pose(
    dbg: DebugContext,
    cycle: Res<Cycle>,
    pose: Res<RobotPose>,
    orientation: Res<RobotOrientation>,
) {
    let orientation = orientation.quaternion();
    let position = pose.inner.translation.vector;
    dbg.log_with_cycle(
        "localization/pose",
        *cycle,
        &rerun::Transform3D::from_rotation(Into::<Quat>::into(orientation))
            .with_translation((position.x, position.y, 0.2865)),
    );
}
