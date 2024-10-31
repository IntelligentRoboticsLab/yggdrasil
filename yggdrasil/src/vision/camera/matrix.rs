use std::marker::PhantomData;

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, CameraPosition};
use nalgebra::{vector, Isometry3, Point2, UnitQuaternion, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    kinematics::{robot_dimensions, RobotKinematics},
    localization::RobotPose,
    motion::walk::{engine::Side, SwingFoot},
    sensor::imu::IMUValues,
};

use super::{init_camera, CameraConfig, Image};

const CAMERA_TOP_PITCH_DEGREES: f32 = 1.2;
const CAMERA_BOTTOM_PITCH_DEGREES: f32 = 39.7;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct CalibrationConfig {
    extrinsic_rotation: Vector3<f32>,
    focal_lengths: Vector2<f32>,
    cc_optical_center: Point2<f32>,
}

#[derive(Default)]
pub struct CameraMatrixPlugin<T: CameraLocation>(PhantomData<T>);

impl<T: CameraLocation> Plugin for CameraMatrixPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMatrix<T>>()
            .add_systems(
                PostStartup,
                setup_camera_matrix_visualization::<T>.after(init_camera::<T>),
            )
            .add_systems(
                Update,
                (update_camera_matrix::<T>, visualize_camera_matrix::<T>)
                    .chain()
                    .after(super::fetch_latest_frame::<T>)
                    .run_if(resource_exists_and_changed::<Image<T>>),
            );
    }
}

fn update_camera_matrix<T: CameraLocation>(
    swing_foot: Res<SwingFoot>,
    imu: Res<IMUValues>,
    kinematics: Res<RobotKinematics>,
    mut matrix: ResMut<CameraMatrix<T>>,
    config: Res<CameraConfig>,
) {
    let config = match T::POSITION {
        CameraPosition::Top => &config.top,
        CameraPosition::Bottom => &config.bottom,
    };

    let image_size = vector![config.width as f32, config.height as f32];
    let camera_to_head = camera_to_head(T::POSITION, config.calibration.extrinsic_rotation);
    *matrix = CameraMatrix::new(
        config.calibration.focal_lengths,
        config.calibration.cc_optical_center,
        image_size,
        camera_to_head,
        kinematics.head_to_robot,
        robot_to_ground(&swing_foot, &imu, &kinematics),
    );
}

fn robot_to_ground(
    swing_foot: &SwingFoot,
    imu: &IMUValues,
    kinematics: &RobotKinematics,
) -> Isometry3<f32> {
    let roll_pitch = imu.angles;
    let roll = roll_pitch.x;
    let pitch = roll_pitch.y;

    let imu_rotation =
        Isometry3::rotation(Vector3::y() * pitch) * Isometry3::rotation(Vector3::x() * roll);

    match swing_foot.support() {
        Side::Left => {
            let left_sole_to_robot = kinematics.left_sole_to_robot;

            imu_rotation * left_sole_to_robot.inverse()
        }
        Side::Right => {
            let right_sole_to_robot = kinematics.right_sole_to_robot;

            imu_rotation * right_sole_to_robot.inverse()
        }
    }
}

fn camera_to_head(position: CameraPosition, extrinsic_rotations: Vector3<f32>) -> Isometry3<f32> {
    // create quaternion, using the extrinsic rotations from config (in degrees!)
    let extrinsic_rotation = UnitQuaternion::from_euler_angles(
        extrinsic_rotations.x.to_radians(),
        extrinsic_rotations.y.to_radians(),
        extrinsic_rotations.z.to_radians(),
    );

    let neck_to_camera = match position {
        CameraPosition::Top => robot_dimensions::NECK_TO_TOP_CAMERA,
        CameraPosition::Bottom => robot_dimensions::NECK_TO_BOTTOM_CAMERA,
    };

    let camera_pitch = match position {
        CameraPosition::Top => CAMERA_TOP_PITCH_DEGREES.to_radians(),
        CameraPosition::Bottom => CAMERA_BOTTOM_PITCH_DEGREES.to_radians(),
    };

    Isometry3::from(neck_to_camera)
        * Isometry3::rotation(Vector3::y() * camera_pitch)
        * extrinsic_rotation
}

fn setup_camera_matrix_visualization<T: CameraLocation>(
    dbg: DebugContext,
    config: Res<CameraConfig>,
) {
    let config = match T::POSITION {
        CameraPosition::Top => &config.top,
        CameraPosition::Bottom => &config.bottom,
    };

    let focal_lengths = (
        config.calibration.focal_lengths.x,
        config.calibration.focal_lengths.y,
    );
    let image_size = (config.width as f32, config.height as f32);

    dbg.log_static(
        T::make_image_entity_path(""),
        &rerun::Pinhole::from_focal_length_and_resolution(focal_lengths, image_size)
            .with_camera_xyz(rerun::components::ViewCoordinates::FLU),
    );
}

fn visualize_camera_matrix<T: CameraLocation>(
    dbg: DebugContext,
    robot_pose: Res<RobotPose>,
    image: Res<Image<T>>,
    matrix: Res<CameraMatrix<T>>,
) {
    let transform = robot_pose.as_3d() * matrix.camera_to_ground;
    let translation = (
        transform.translation.x,
        transform.translation.y,
        transform.translation.z,
    );
    let rotation = &transform.rotation.coords;
    let quaternion = [rotation.x, rotation.y, rotation.z, rotation.w];

    dbg.log_with_cycle(
        T::make_image_entity_path(""),
        image.cycle(),
        &rerun::Transform3D::from_translation(translation).with_quaternion(quaternion),
    );
}
