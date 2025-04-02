use std::marker::PhantomData;

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, CameraPosition};
use nalgebra::{vector, Isometry3, Point2, UnitQuaternion, Vector2, Vector3};
use rerun::external::glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    kinematics::{
        dimensions,
        spaces::{Head, Left, Right, Robot, Sole},
        Kinematics,
    },
    localization::RobotPose,
    motion::walking_engine::{foot_support::FootSupportState, Side},
    nao::Cycle,
    sensor::orientation::RobotOrientation,
};

use super::CameraConfig;

const CAMERA_TOP_PITCH_DEGREES: f32 = 1.2;
const CAMERA_BOTTOM_PITCH_DEGREES: f32 = 39.7;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
pub struct CalibrationConfig {
    pub extrinsic_rotation: Vector3<f32>,
    focal_lengths: Vector2<f32>,
    cc_optical_center: Point2<f32>,
}

#[derive(Default)]
pub struct CameraMatrixPlugin<T: CameraLocation>(PhantomData<T>);

impl<T: CameraLocation> Plugin for CameraMatrixPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraMatrix<T>>()
            .add_systems(PostStartup, setup_camera_matrix_visualization::<T>)
            .add_systems(
                Update,
                (
                    update_camera_matrix::<T>.before(super::fetch_latest_frame::<T>),
                    visualize_camera_matrix::<T>,
                )
                    .chain(),
            );
    }
}

fn update_camera_matrix<T: CameraLocation>(
    foot_support: Res<FootSupportState>,
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
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
        kinematics.isometry::<Head, Robot>().inner,
        robot_to_ground(foot_support.support_side(), &orientation, &kinematics),
    );
}

fn robot_to_ground(
    support_side: Side,
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> Isometry3<f32> {
    let (roll, pitch, _) = orientation.euler_angles();

    let left_sole_to_robot = kinematics.isometry::<Sole<Left>, Robot>().inner;
    let imu_adjusted_robot_to_left_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(vector![0., 0., -left_sole_to_robot.translation.z]);

    let right_sole_to_robot = kinematics.isometry::<Sole<Right>, Robot>().inner;
    let imu_adjusted_robot_to_right_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(vector![0., 0., -right_sole_to_robot.translation.z]);

    match support_side {
        Side::Left => imu_adjusted_robot_to_left_sole,
        Side::Right => imu_adjusted_robot_to_right_sole,
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
        CameraPosition::Top => dimensions::NECK_TO_TOP_CAMERA,
        CameraPosition::Bottom => dimensions::NECK_TO_BOTTOM_CAMERA,
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

    let pinhole = rerun::Pinhole::from_focal_length_and_resolution(
        [
            config.calibration.focal_lengths.x,
            config.calibration.focal_lengths.y,
        ],
        [config.width as f32, config.height as f32],
    )
    .with_camera_xyz(rerun::components::ViewCoordinates::FLU)
    .with_image_plane_distance(0.35);

    dbg.log_static(T::make_entity_image_path(""), &pinhole);
}

fn visualize_camera_matrix<T: CameraLocation>(
    dbg: DebugContext,
    matrix: Res<CameraMatrix<T>>,
    cycle: Res<Cycle>,
    pose: Res<RobotPose>,
) {
    let camera_pos = pose.to_3d() * matrix.camera_to_ground;

    dbg.log_with_cycle(
        T::make_entity_image_path(""),
        *cycle,
        &rerun::Transform3D::update_fields()
            .with_translation(Into::<Vec3>::into(camera_pos.translation))
            .with_quaternion(Into::<Quat>::into(camera_pos.rotation)),
    );
}
