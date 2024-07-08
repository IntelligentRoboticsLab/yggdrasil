use crate::{
    behavior::primary_state::PrimaryState,
    core::{
        config::{layout::LayoutConfig, showtime::PlayerConfig},
        debug::DebugContext,
    },
    motion::odometry::{self, Odometry},
    prelude::*,
};
use nalgebra::{Isometry2, Point2, Translation2, UnitComplex};
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
    let position = Point2::new(initial_position.x, initial_position.y);
    let orientation = initial_position.rotation.to_radians();

    let initial_pose = Isometry2::from_parts(
        Translation2::from(position),
        UnitComplex::from_angle(orientation),
    );

    storage.add_resource(Resource::new(RobotPose::new(initial_pose)))?;

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
    if *primary_state == PrimaryState::Penalized {
        if let Some(closest_penalty_pose) = find_closest_penalty_pose(robot_pose, layout_config) {
            robot_pose.inner = closest_penalty_pose;
        }
    }
    robot_pose.inner *= odometry.offset_to_last;
    log_pose(
        "/localisation/pose",
        ctx,
        &robot_pose.inner,
        color::u8::BLUE,
    )?;
    Ok(())
}

fn find_closest_penalty_pose(
    robot_pose: &RobotPose,
    layout_config: &LayoutConfig,
) -> Option<Isometry2<f32>> {
    let penalty_poses = layout_config
        .penalty_positions
        .iter()
        .map(|penalty_position| {
            Isometry2::from_parts(
                Translation2::new(penalty_position.x, penalty_position.y),
                UnitComplex::from_angle(penalty_position.rotation.to_radians()),
            )
        });

    penalty_poses.min_by_key(|penalty_pose| {
        let distance =
            (robot_pose.inner.translation.vector - penalty_pose.translation.vector).norm_squared();
        distance as i32
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
