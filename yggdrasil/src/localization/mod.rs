use crate::{
    behavior::primary_state::PrimaryState,
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig},
        debug::DebugContext,
    },
    motion::odometry::{self, Odometry},
    prelude::*,
};
use nalgebra::{Isometry2, Isometry3, Point2, Translation3, UnitQuaternion};
use nidhogg::types::{
    color::{self, RgbU8},
    HeadJoints,
};

pub struct LocalizationModule;

impl Module for LocalizationModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(update_robot_pose.after(odometry::update_odometry))
            .add_startup_system(init_pose)
    }
}

#[startup_system]
fn init_pose(
    storage: &mut Storage,
    layout_config: &LayoutConfig,
    player_config: &PlayerConfig,
) -> Result<()> {
    let initial_position = layout_config
        .initial_positions
        .player(player_config.player_number);

    storage.add_resource(Resource::new(RobotPose::new(initial_position.isometry)))?;

    Ok(())
}

#[derive(Default, Debug, Clone)]
pub struct RobotPose {
    pub inner: Isometry2<f32>,
}

impl RobotPose {
    fn new(pose: Isometry2<f32>) -> Self {
        Self { inner: pose }
    }

    /// The current pose of the robot in the world, in 3D space.
    ///
    /// The z-axis is always 0.
    /// The rotation is around the z-axis.
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
    pub fn world_position(&self) -> Point2<f32> {
        self.inner.translation.vector.into()
    }

    /// The current rotation of the robot in the world, in radians.
    pub fn world_rotation(&self) -> f32 {
        self.inner.rotation.angle()
    }

    /// Transform a point from robot coordinates to world coordinates.
    pub fn robot_to_world(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.transform_point(point)
    }

    /// Transform a point from world coordinates to robot coordinates.
    pub fn world_to_robot(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.inverse_transform_point(point)
    }

    pub fn get_look_at_absolute(&self, point_in_world: &Point2<f32>) -> HeadJoints<f32> {
        let robot_to_point = self.world_to_robot(point_in_world).xy();
        self.get_look_at(&robot_to_point)
    }

    pub fn get_look_at(&self, robot_to_point: &Point2<f32>) -> HeadJoints<f32> {
        let yaw = (robot_to_point.y / robot_to_point.x).atan();
        // This cannot be computed without properly turning it into a 3d point by e.g. projecting it, but
        // that's for later
        // let pitch = (robot_to_point.z / robot_to_point.magnitude).acos();

        HeadJoints { yaw, pitch: 0.0 }
    }
}

#[system]
pub fn update_robot_pose(
    robot_pose: &mut RobotPose,
    odometry: &Odometry,
    ctx: &DebugContext,
    primary_state: &PrimaryState,
    layout_config: &LayoutConfig,
) -> Result<()> {
    *robot_pose = next_robot_pose(robot_pose, odometry, primary_state, layout_config);
    log_pose(
        "/localisation/pose",
        ctx,
        &robot_pose.inner,
        color::u8::BLUE,
    )?;
    Ok(())
}

pub fn next_robot_pose(
    robot_pose: &RobotPose,
    odometry: &Odometry,
    primary_state: &PrimaryState,
    layout_config: &LayoutConfig,
) -> RobotPose {
    let isometry = if *primary_state == PrimaryState::Penalized {
        find_closest_penalty_pose(robot_pose, layout_config)
    } else {
        robot_pose.inner * odometry.offset_to_last
    };

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

            match distance_b > distance_a {
                true => a,
                false => b,
            }
        })
        .unwrap_or_else(|| {
            tracing::warn!("Failed to find closest penalty pose for");
            &robot_pose.inner
        })
}

fn log_pose(
    path: impl AsRef<str>,
    ctx: &DebugContext,
    pose: &Isometry2<f32>,
    color: RgbU8,
) -> Result<()> {
    let origin = pose.translation.vector;
    let direction = pose.rotation.transform_point(&Point2::new(0.1, 0.0));

    ctx.log_arrows3d_with_color(
        path,
        &[(direction.x, direction.y, 0.0)],
        &[(origin.x, origin.y, 0.0)],
        color,
    )
}
