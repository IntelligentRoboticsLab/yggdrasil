use bevy::prelude::*;
use nalgebra::{Point2, Vector2};
use nidhogg::types::{Fsr, FsrFoot};

use crate::sensor::imu::IMUValues;

use super::center_of_mass;

/// Struct containing offsets of each FSR sensor relative to the center of the robot's foot.
///
/// See <http://doc.aldebaran.com/2-8/family/nao_technical/fsr_naov6.html> for more.
#[derive(Debug, Default, Clone, Copy)]
struct SensorOffsets {
    front_left: Vector2<f32>,
    front_right: Vector2<f32>,
    rear_left: Vector2<f32>,
    rear_right: Vector2<f32>,
}

/// Offsets for the FSR sensors in the left foot of the robot, values taken from the NAO urdf.
const LEFT_FOOT_SENSOR_OFFSETS: SensorOffsets = SensorOffsets {
    front_left: Vector2::new(0.07025, -0.0299),
    front_right: Vector2::new(0.07025, 0.0299),
    rear_left: Vector2::new(-0.03025, -0.0299),
    rear_right: Vector2::new(-0.03025, 0.0299),
};

/// Offsets for the FSR sensors in the right foot of the robot, values taken from the NAO urdf.
const RIGHT_FOOT_SENSOR_OFFSETS: SensorOffsets = SensorOffsets {
    front_left: Vector2::new(0.07025, -0.0231),
    front_right: Vector2::new(0.07025, 0.0299),
    rear_left: Vector2::new(-0.03025, -0.0191),
    rear_right: Vector2::new(-0.03025, 0.0299),
};

const GRAVITY_CONSTANT: f32 = 9.81;

/// Plugin that adds systems and resources for calculating the center of pressure.
pub(super) struct CenterOfPressurePlugin;

impl Plugin for CenterOfPressurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CenterOfPressure>()
            .init_resource::<ZeroMomentPoint>();
        app.add_systems(
            Update,
            (update_cop, update_zmp.after(center_of_mass::update_com)),
        );
    }
}

#[derive(Debug, Clone, Resource)]
pub struct CenterOfPressure {
    pub left: Vector2<f32>,
    pub right: Vector2<f32>,
}

impl Default for CenterOfPressure {
    fn default() -> Self {
        CenterOfPressure {
            left: Vector2::new(0.0, 0.0),
            right: Vector2::new(0.0, 0.0),
        }
    }
}

fn update_cop(mut center_of_pressure: ResMut<CenterOfPressure>, fsr: Res<Fsr>) {
    center_of_pressure.left = compute_center_of_pressure(&fsr.left_foot, LEFT_FOOT_SENSOR_OFFSETS);
    center_of_pressure.right =
        compute_center_of_pressure(&fsr.right_foot, RIGHT_FOOT_SENSOR_OFFSETS);
}

fn compute_center_of_pressure(fsr_foot: &FsrFoot, offsets: SensorOffsets) -> Vector2<f32> {
    (fsr_foot.front_left * offsets.front_left
        + fsr_foot.front_right * offsets.front_right
        + fsr_foot.rear_left * offsets.rear_left
        + fsr_foot.rear_right * offsets.rear_right)
        / fsr_foot.sum()
}

/// Resource containing the current Zero Moment Point (ZMP) of the robot.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ZeroMomentPoint {
    pub point: Point2<f32>,
}

fn update_zmp(
    mut zero_moment_point: ResMut<ZeroMomentPoint>,
    imu: Res<IMUValues>,
    center_of_mass: Res<center_of_mass::CenterOfMass>,
) {
    let accel_x = imu.accelerometer.x;
    let accel_y = imu.accelerometer.y;

    let x_com = center_of_mass.position.x;
    let y_com = center_of_mass.position.y;
    let z_com = center_of_mass.position.z;

    zero_moment_point.point = Point2::new(
        x_com - (z_com / GRAVITY_CONSTANT) * accel_x,
        y_com - (z_com / GRAVITY_CONSTANT) * accel_y,
    );
}
