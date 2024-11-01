use std::marker::PhantomData;

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, CameraPosition};
use nalgebra::{vector, Isometry3, Point2, Point3, UnitQuaternion, Vector2, Vector3};
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
    dbg: DebugContext,
    pose: Res<RobotPose>,
    swing_foot: Res<SwingFoot>,
    imu: Res<IMUValues>,
    kinematics: Res<RobotKinematics>,
    mut matrix: ResMut<CameraMatrix<T>>,
    image: Res<Image<T>>,
    config: Res<CameraConfig>,
) {
    let config = match T::POSITION {
        CameraPosition::Top => &config.top,
        CameraPosition::Bottom => &config.bottom,
    };

    let image_size = vector![config.width as f32, config.height as f32];
    let camera_to_neck = camera_to_neck(T::POSITION, config.calibration.extrinsic_rotation);
    let camera_to_robot = kinematics.head_to_robot * camera_to_neck;
    let robot_to_ground = robot_to_ground(&swing_foot, &imu, &kinematics);

    info!("== {:?} ==", T::POSITION);
    info!("camera_to_neck: {:?}", camera_to_neck);
    info!("neck_to_robot: {:?}", kinematics.neck_to_robot);
    info!("head_to_robot: {:?}", kinematics.head_to_robot);
    info!("robot_to_ground: {:?}", robot_to_ground);

    *matrix = CameraMatrix::new(
        config.calibration.focal_lengths,
        config.calibration.cc_optical_center,
        image_size,
        camera_to_neck,
        kinematics.head_to_robot,
        robot_to_ground,
    );

    let test_point = Point3::new(1.0, 0.0, 0.0); // 1 meter
    match matrix.ground_to_pixel(test_point) {
        Ok(pixel) => dbg.log_with_cycle(
            T::make_image_entity_path("test_point"),
            image.cycle(),
            &rerun::Points2D::new(&[(pixel.x, pixel.y)]),
        ),
        Err(e) => error!(?e, "Failed to project test point"),
    }

    let test_pixel = Point2::new((image.width() as f32 / 2.0), (image.height() as f32 / 2.0));
    match matrix.pixel_to_ground(test_pixel, 0.0) {
        Ok(point) => {
            // let transformed = pose.as_3d().transform_point(&point);
            dbg.log_with_cycle(
                T::make_entity_path("test_pixel"),
                image.cycle(),
                &rerun::Points3D::new(&[(point.x, point.y, point.z)]),
            )
        }
        Err(e) => error!(?e, "Failed to project test pixel"),
    }
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
        Isometry3::rotation(Vector3::x() * roll) * Isometry3::rotation(Vector3::y() * pitch);

    let sole_to_robot = match swing_foot.support() {
        Side::Left => kinematics.left_sole_to_robot,
        Side::Right => kinematics.right_sole_to_robot,
    };

    let robot_to_ground = imu_rotation.inverse() * sole_to_robot.inverse();
    robot_to_ground
}

fn camera_to_neck(position: CameraPosition, extrinsic_rotations: Vector3<f32>) -> Isometry3<f32> {
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

    // First, translate from neck to camera
    let translation = Isometry3::translation(neck_to_camera.x, neck_to_camera.y, neck_to_camera.z);

    // Then apply fixed camera pitch rotation
    let rotation = Isometry3::rotation(Vector3::y() * camera_pitch);

    // Apply extrinsic calibration rotation
    let extrinsic = Isometry3::from_parts(nalgebra::Translation3::identity(), extrinsic_rotation);

    // Combine transformations: Translation -> Fixed Pitch -> Extrinsic Rotation
    let neck_to_camera = translation * rotation * extrinsic;

    // Since you need camera_to_neck, invert the transformation
    neck_to_camera.inverse()
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
        &rerun::Transform3D::from_translation(translation),
    );
}
