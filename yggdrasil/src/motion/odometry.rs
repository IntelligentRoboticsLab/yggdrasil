use nalgebra::{Isometry2, Translation2, UnitComplex, Vector2};
use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use crate::{
    debug::DebugContext,
    kinematics::RobotKinematics,
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

        // TODO: Use the IMU to correct orientation
        let odometry_offset =
            Isometry2::from_parts(Translation2::from(scaled_offset), UnitComplex::identity());

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
) -> Result<()> {
    odometry.update(odometry_config, swing_foot, kinematics);
    Ok(())
}

#[system]
pub fn log_odometry(odometry: &Odometry, dbg: &DebugContext) -> Result<()> {
    let accumulated = odometry.accumulated.translation;
    let accumulated = (accumulated.x, accumulated.y, 0.0);
    dbg.log_points_3d_with_color_and_radius(
        "/odometry/accumulated",
        &[accumulated],
        color::u8::RED,
        0.05,
    )?;
    Ok(())
}
