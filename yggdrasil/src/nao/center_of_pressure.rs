use bevy::prelude::*;
use nalgebra::{Point2, Vector2};
use nidhogg::types::{Fsr, FsrFoot};

use crate::sensor::imu::IMUValues;

use super::center_of_mass;

type SensorOffsets = (Vector2<f32>, Vector2<f32>, Vector2<f32>, Vector2<f32>);
const LEFT_FOOT_SENSOR_OFFSETS: SensorOffsets = (
    Vector2::new(0.07025, -0.0299),
    Vector2::new(0.07025, 0.0299),
    Vector2::new(-0.03025, -0.0299),
    Vector2::new(-0.03025, 0.0299),
);

const RIGHT_FOOT_SENSOR_OFFSETS: SensorOffsets = (
    Vector2::new(0.07025, -0.0231),
    Vector2::new(0.07025, 0.0299),
    Vector2::new(-0.03025, -0.0191),
    Vector2::new(-0.03025, 0.0299),
);

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
    let (front_left, front_right, rear_left, rear_right) = offsets;
    (fsr_foot.front_left * front_left
        + fsr_foot.front_right * front_right
        + fsr_foot.rear_left * rear_left
        + fsr_foot.rear_right * rear_right)
        / fsr_foot.sum()
}

#[derive(Debug, Clone, Resource)]
pub struct ZeroMomentPoint {
    pub point: Point2<f32>,
}

impl Default for ZeroMomentPoint {
    fn default() -> Self {
        ZeroMomentPoint {
            point: Point2::new(0.0, 0.0),
        }
    }
}

fn update_zmp(
    mut zero_moment_point: ResMut<ZeroMomentPoint>,
    imu: Res<IMUValues>,
    center_of_mass: Res<center_of_mass::CenterOfMass>,
) {
    let ddx = imu.accelerometer.x;
    let ddy = imu.accelerometer.y;

    let x_com = center_of_mass.position.x;
    let y_com = center_of_mass.position.y;
    let z_com = center_of_mass.position.z;

    zero_moment_point.point = Point2::new(
        x_com - (z_com / GRAVITY_CONSTANT) * ddx,
        y_com - (z_com / GRAVITY_CONSTANT) * ddy,
    );
}
