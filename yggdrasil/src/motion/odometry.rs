use nalgebra::{Isometry2, Point2, Translation2, UnitComplex, Vector2};
use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        layout::{LayoutConfig, RobotPosition},
        showtime::ShowtimeConfig,
    },
    debug::DebugContext,
    filter::orientation::RobotOrientation,
    kinematics::RobotKinematics,
    nao::RobotInfo,
    prelude::*,
    walk::{engine::Side, SwingFoot},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdometryConfig {
    scale_factor: Vector2<f32>,
}

#[derive(Debug, Default, Clone)]
pub struct Odometry {
    pub accumulated: Isometry2<f32>,
    last_left_sole_to_right_sole: Vector2<f32>,
    last_orientation: UnitComplex<f32>,
}

impl Odometry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the odometry of the robot using the given [`RobotKinematics`].
    pub fn update(
        &mut self,
        config: &OdometryConfig,
        swing_foot: &SwingFoot,
        kinematics: &RobotKinematics,
        orientation: &RobotOrientation,
    ) {
        let left_sole_to_robot = kinematics.left_sole_to_robot;
        let right_sole_to_robot = kinematics.right_sole_to_robot;

        let left_sole_to_right_sole =
            (right_sole_to_robot.translation.vector - left_sole_to_robot.translation.vector).xy();

        // Compute offset to last position, divided by 2 to get the center of the robot.
        let offset = match swing_foot.support() {
            Side::Left => left_sole_to_right_sole - self.last_left_sole_to_right_sole,
            Side::Right => -left_sole_to_right_sole + self.last_left_sole_to_right_sole,
        } / 2.0;

        self.last_left_sole_to_right_sole = left_sole_to_right_sole;
        let scaled_offset = offset.component_mul(&config.scale_factor);

        let orientation_offset = self
            .last_orientation
            .rotation_to(&orientation.yaw())
            .inverse();
        self.last_orientation = orientation.yaw();

        let odometry_offset =
            Isometry2::from_parts(Translation2::from(scaled_offset), orientation_offset);

        // update the accumulated odometry
        self.accumulated *= odometry_offset;
    }
}

#[system]
pub fn update_odometry(
    odometry: &mut Odometry,
    odometry_config: &OdometryConfig,
    swing_foot: &SwingFoot,
    kinematics: &RobotKinematics,
    orientation: &RobotOrientation,
) -> Result<()> {
    odometry.update(odometry_config, swing_foot, kinematics, orientation);
    Ok(())
}

pub fn isometry_to_absolute(
    isometry: Isometry2<f32>,
    robot_position: &RobotPosition,
) -> Isometry2<f32> {
    Isometry2::new(
        Vector2::new(robot_position.x, robot_position.y),
        robot_position.rotation,
    ) * isometry
}

#[system]
pub fn log_odometry(
    odometry: &Odometry,
    layout_config: &LayoutConfig,
    dbg: &DebugContext,
    showtime_config: &ShowtimeConfig,
    robot_info: &RobotInfo,
) -> Result<()> {
    let player_num = showtime_config.robot_numbers_map[&robot_info.robot_id.to_string()];
    let isometry = isometry_to_absolute(
        odometry.accumulated,
        layout_config.initial_positions.player(player_num),
    );

    let rotated = isometry.rotation.transform_point(&Point2::new(0.1, 0.0));
    let origin = isometry.translation.transform_point(&Point2::origin());

    dbg.log_arrows3d_with_color(
        "/odometry/pose",
        &[(rotated.x, rotated.y, 0.0)],
        &[(origin.x, origin.y, 0.0)],
        color::u8::RED,
    )?;

    Ok(())
}

#[startup_system]
pub(super) fn setup_viewcoordinates(_storage: &mut Storage, dbg: &DebugContext) -> Result<()> {
    dbg.log_robot_viewcoordinates("/odometry/pose")?;
    Ok(())
}
