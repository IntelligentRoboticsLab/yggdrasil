pub mod correspondence;
pub mod pose_filter;

use std::f32::consts::PI;

use crate::{
    behavior::primary_state::PrimaryState,
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig},
        debug::DebugContext,
    },
    motion::odometry::{self, Odometry},
    nao::Cycle,
    sensor::orientation::RobotOrientation,
};
use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, GamePhase};
use correspondence::LineCorrespondencePlugin;
use heimdall::{Bottom, Top};
use nalgebra::{
    Isometry2, Isometry3, Point2, Point3, Translation2, Translation3, UnitComplex, UnitQuaternion,
};
use nidhogg::types::HeadJoints;
use pose_filter::PoseFilterPlugin;
use rerun::{external::glam::Quat, TimeColumn};

/// The localization plugin provides functionalities related to the localization of the robot.
pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            LineCorrespondencePlugin::<Top>::default(),
            // LineCorrespondencePlugin::<Bottom>::default(),
            PoseFilterPlugin,
        ))
        .add_systems(PostStartup, (init_pose, setup_pose_visualization))
        .add_systems(
            PreUpdate,
            update_robot_pose.after(odometry::update_odometry),
        )
        .add_systems(PostUpdate, visualize_pose);
    }
}

fn init_pose(
    mut commands: Commands,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
) {
    let initial_position = layout_config
        .initial_positions
        .player(player_config.player_number);

    commands.insert_resource(RobotPose::new(initial_position.isometry));
}

#[derive(Resource, Default, Debug, Clone)]
pub struct RobotPose {
    pub inner: Isometry2<f32>,
}

impl RobotPose {
    // Constant for camera height that we set anywhere get_lookat_absolute is called.
    // Set to zero if we are only looking at the ground, for example.
    pub const CAMERA_HEIGHT: f32 = 0.5;

    fn new(pose: Isometry2<f32>) -> Self {
        Self { inner: pose }
    }

    /// The current pose of the robot in the world, in 3D space.
    ///
    /// The z-axis is always 0.
    /// The rotation is around the z-axis.
    #[must_use]
    pub fn as_3d(&self) -> Isometry3<f32> {
        Isometry3::from_parts(
            Translation3::new(self.inner.translation.x, self.inner.translation.y, 0.0),
            UnitQuaternion::from_euler_angles(0.0, 0.0, self.inner.rotation.angle()),
        )
    }

    /// The current position of the robot in the world, in absolute coordinates.
    ///
    /// The center of the world is at the center of the field, with the x-axis pointing towards the
    /// opponent's goal.
    #[must_use]
    pub fn world_position(&self) -> Point2<f32> {
        self.inner.translation.vector.into()
    }

    /// The current rotation of the robot in the world, in radians.
    #[must_use]
    pub fn world_rotation(&self) -> f32 {
        self.inner.rotation.angle()
    }

    /// Transform a point from robot coordinates to world coordinates.
    #[must_use]
    pub fn robot_to_world(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.transform_point(point)
    }

    /// Transform a point from world coordinates to robot coordinates.
    #[must_use]
    pub fn world_to_robot(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.inverse_transform_point(point)
    }

    #[must_use]
    pub fn get_look_at_absolute(&self, point_in_world: &Point3<f32>) -> HeadJoints<f32> {
        let robot_to_point = self.world_to_robot(&point_in_world.xy());
        let x = robot_to_point.x;
        let y = robot_to_point.y;
        let z = point_in_world.z;
        let yaw = (robot_to_point.y / robot_to_point.x).atan();
        // 0.5 is the height of the robot's primary camera while standing
        let pitch = (0.5 - z).atan2((x * x + y * y).sqrt());

        HeadJoints { yaw, pitch }
    }

    #[must_use]
    pub fn distance_to(&self, point: &Point2<f32>) -> f32 {
        (self.world_position() - point).norm()
    }

    #[must_use]
    pub fn angle_to(&self, point: &Point2<f32>) -> f32 {
        let robot_to_point = self.world_to_robot(point).xy();
        robot_to_point.y.atan2(robot_to_point.x)
    }
}

fn update_robot_pose(
    mut robot_pose: ResMut<RobotPose>,
    odometry: Res<Odometry>,
    primary_state: Res<PrimaryState>,
    layout_config: Res<LayoutConfig>,
    game_controller_message: Option<Res<GameControllerMessage>>,
) {
    *robot_pose = next_robot_pose(
        robot_pose.as_mut(),
        odometry.as_ref(),
        primary_state.as_ref(),
        layout_config.as_ref(),
        game_controller_message.as_deref(),
    );
}

#[must_use]
pub fn next_robot_pose(
    robot_pose: &RobotPose,
    odometry: &Odometry,
    primary_state: &PrimaryState,
    layout_config: &LayoutConfig,
    message: Option<&GameControllerMessage>,
) -> RobotPose {
    let mut isometry = if *primary_state == PrimaryState::Penalized {
        find_closest_penalty_pose(robot_pose, layout_config)
    } else {
        robot_pose.inner * odometry.offset_to_last
    };

    if let Some(message) = message {
        if message.game_phase == GamePhase::PenaltyShoot {
            if message.kicking_team == 8 {
                isometry = Isometry2::from_parts(
                    Translation2::new(3.2, 0.0),
                    UnitComplex::from_angle(0.0),
                );
            } else {
                isometry =
                    Isometry2::from_parts(Translation2::new(4.5, 0.0), UnitComplex::from_angle(PI));
            }
        }
    }

    RobotPose::new(isometry)
}

fn find_closest_penalty_pose(
    robot_pose: &RobotPose,
    layout_config: &LayoutConfig,
) -> Isometry2<f32> {
    *layout_config
        .penalty_positions
        .iter()
        .reduce(|a, b| {
            let distance_a =
                (robot_pose.inner.translation.vector - a.translation.vector).norm_squared();
            let distance_b =
                (robot_pose.inner.translation.vector - b.translation.vector).norm_squared();

            if distance_b > distance_a {
                a
            } else {
                b
            }
        })
        .unwrap_or_else(|| {
            tracing::warn!("failed to find closest penalty pose for");
            &robot_pose.inner
        })
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
