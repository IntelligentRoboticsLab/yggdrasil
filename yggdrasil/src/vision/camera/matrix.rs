use std::{marker::PhantomData, ops::Deref};

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix, CameraPosition};
use nalgebra::{vector, Isometry3, Point2, Point3, UnitQuaternion, Vector2, Vector3};
use rerun::external::glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::{
    core::debug::DebugContext,
    kinematics::{
        dimensions,
        spaces::{
            ChestCentreLeft, ChestCentreRight, ChestLeft, ChestRight, Head, LeftToe, RightToe,
            Robot,
        },
        Kinematics,
    },
    localization::RobotPose,
    motion::walk::{engine::Side, SwingFoot},
    nao::Cycle,
    sensor::orientation::RobotOrientation,
};

use super::{CameraConfig, Image};

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
            .add_systems(PostStartup, setup_body_contour_visualization::<Bottom>)
            .add_systems(
                Update,
                (
                    update_camera_matrix::<T>.before(super::fetch_latest_frame::<T>),
                    visualize_camera_matrix::<T>,
                )
                    .chain(),
            )
            .add_systems(Update, print_toes::<Bottom>)
            .add_systems(Update, print_chest::<Bottom>);
        // .add_systems(Update, print_shoulders::<Bottom>);
    }
}

fn setup_body_contour_visualization<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_component_batches(
        T::make_entity_path("body_contour/toes"),
        true,
        [
            &rerun::Color::from_rgb(219, 62, 177) as _,
            &rerun::Radius::new_ui_points(14.0) as _,
        ],
    );

    dbg.log_component_batches(
        T::make_entity_path("body_contour/chest"),
        true,
        [
            &rerun::Color::from_rgb(255, 255, 0) as _,
            &rerun::Radius::new_ui_points(14.0) as _,
        ],
    );

    dbg.log_component_batches(
        T::make_entity_path("body_contour/shoulders"),
        true,
        [
            &rerun::Color::from_rgb(0, 238, 255) as _,
            &rerun::Radius::new_ui_points(14.0) as _,
        ],
    );
}

fn update_camera_matrix<T: CameraLocation>(
    swing_foot: Res<SwingFoot>,
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
        robot_to_ground(&swing_foot, &orientation, &kinematics),
    );
}

fn print_toes<T: CameraLocation>(
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
    debug_context: DebugContext,
    matrix: Res<CameraMatrix<T>>,
    bottom_image: Res<Image<T>>,
    current_cycle: Res<Cycle>,
) {
    if !bottom_image.is_from_cycle(*current_cycle) {
        return;
    }

    let (robot_to_left_toe, robot_to_right_toe) = robot_to_toes(&orientation, &kinematics);

    let (Ok(left_toe_point), Ok(right_toe_point)) = (
        matrix.ground_to_pixel(
            (robot_to_left_toe.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
        matrix.ground_to_pixel(
            (robot_to_right_toe.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
    ) else {
        return;
    };

    debug_context.log_with_cycle(
        T::make_entity_path("body_contour/toes"),
        bottom_image.deref().cycle(),
        &rerun::Points2D::new([
            (left_toe_point.x, left_toe_point.y),
            (right_toe_point.x, right_toe_point.y),
        ]),
    );
}

fn print_chest<T: CameraLocation>(
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
    debug_context: DebugContext,
    matrix: Res<CameraMatrix<T>>,
    current_cycle: Res<Cycle>,
) {
    let (
        robot_to_chest_left,
        robot_to_chest_centre_left,
        robot_to_chest_centre_right,
        robot_to_chest_right,
    ) = robot_to_chests(&orientation, &kinematics);

    let (
        Ok(chest_left_point),
        Ok(chest_centre_left_point),
        Ok(chest_centre_right_point),
        Ok(chest_right_point),
    ) = (
        matrix.ground_to_pixel(
            (robot_to_chest_left.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
        matrix.ground_to_pixel(
            (robot_to_chest_centre_left.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
        matrix.ground_to_pixel(
            (robot_to_chest_centre_right.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
        matrix.ground_to_pixel(
            (robot_to_chest_right.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ),
    )
    else {
        return;
    };

    debug_context.log_with_cycle(
        T::make_entity_path("body_contour/chest"),
        *current_cycle,
        &rerun::Points2D::new([
            (chest_left_point.x, chest_left_point.y),
            (chest_centre_left_point.x, chest_centre_left_point.y),
            (chest_centre_right_point.x, chest_centre_right_point.y),
            (chest_right_point.x, chest_right_point.y),
        ]),
    );
}

// fn print_shoulders<T: CameraLocation>(
//     orientation: Res<RobotOrientation>,
//     kinematics: RobotKinematics,
//     debug_context: DebugContext,
//     matrix: Res<CameraMatrix<T>>,
//     current_cycle: Res<Cycle>,
// ) {
//     let (
//         left_shoulder_cap_back_to_robot,
//         left_shoulder_cap_front_to_robot,
//         right_shoulder_cap_front_to_robot,
//         right_shoulder_cap_back_to_robot,
//     ) = robot_to_shoulders(&orientation, &kinematics);
//
//     eprintln!("robot_to_left_shoulder_cap_back: {left_shoulder_cap_back_to_robot}");
//     eprintln!("robot_to_left_shoulder_cap_front: {left_shoulder_cap_front_to_robot}");
//
//     let (
//         Ok(left_shoulder_cap_back_point),
//         Ok(left_shoulder_cap_front_point),
//         Ok(right_shoulder_cap_front_point),
//         Ok(right_shoulder_cap_back_point),
//     ) = (
//         matrix.ground_to_pixel(
//             (left_shoulder_cap_back_to_robot * matrix.robot_to_ground)
//                 .inverse()
//                 .translation
//                 .vector
//                 .into(),
//         ),
//         matrix.ground_to_pixel(
//             (left_shoulder_cap_front_to_robot * matrix.robot_to_ground)
//                 .inverse()
//                 .translation
//                 .vector
//                 .into(),
//         ),
//         matrix.ground_to_pixel(
//             (right_shoulder_cap_front_to_robot * matrix.robot_to_ground)
//                 .inverse()
//                 .translation
//                 .vector
//                 .into(),
//         ),
//         matrix.ground_to_pixel(
//             (right_shoulder_cap_back_to_robot * matrix.robot_to_ground)
//                 .inverse()
//                 .translation
//                 .vector
//                 .into(),
//         ),
//     )
//     else {
//         return;
//     };
//
//     debug_context.log_with_cycle(
//         T::make_entity_path("body_contour/shoulders"),
//         *current_cycle,
//         &rerun::Points2D::new([
//             (
//                 left_shoulder_cap_back_point.x,
//                 left_shoulder_cap_back_point.y,
//             ),
//             (
//                 left_shoulder_cap_front_point.x,
//                 left_shoulder_cap_front_point.y,
//             ),
//             (
//                 right_shoulder_cap_front_point.x,
//                 right_shoulder_cap_front_point.y,
//             ),
//             (
//                 right_shoulder_cap_back_point.x,
//                 right_shoulder_cap_back_point.y,
//             ),
//         ]),
//     );
// }

fn robot_to_ground(
    swing_foot: &SwingFoot,
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> Isometry3<f32> {
    let (roll, pitch, _) = orientation.euler_angles();

    let left_sole_to_robot = kinematics.isometry::<LeftToe, Robot>().inner;
    let imu_adjusted_robot_to_left_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(vector![0., 0., -left_sole_to_robot.translation.z]);

    let right_sole_to_robot = kinematics.isometry::<RightToe, Robot>().inner;
    let imu_adjusted_robot_to_right_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(vector![0., 0., -right_sole_to_robot.translation.z]);

    match swing_foot.support() {
        Side::Left => imu_adjusted_robot_to_left_sole,
        Side::Right => imu_adjusted_robot_to_right_sole,
    }
}

fn robot_to_toes(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let left_toe_to_robot = kinematics.isometry::<LeftToe, Robot>().inner;
    let imu_adjusted_robot_to_left_toe = Isometry3::from(left_toe_to_robot.translation.inverse())
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let right_toe_to_robot = kinematics.isometry::<RightToe, Robot>().inner;
    let imu_adjusted_robot_to_right_toe = Isometry3::from(right_toe_to_robot.translation.inverse())
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_left_toe,
        imu_adjusted_robot_to_right_toe,
    )
}

fn robot_to_chests(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (
    Isometry3<f32>,
    Isometry3<f32>,
    Isometry3<f32>,
    Isometry3<f32>,
) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_chest_left = kinematics.isometry::<Robot, ChestLeft>().inner;
    let imu_adjusted_chest_left_to_robot = Isometry3::from(robot_to_chest_left.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_chest_centre_left = kinematics.isometry::<Robot, ChestCentreLeft>().inner;
    let imu_adjusted_chest_centre_left_to_robot =
        Isometry3::from(robot_to_chest_centre_left.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_chest_centre_right = kinematics.isometry::<Robot, ChestCentreRight>().inner;
    let imu_adjusted_chest_centre_right_to_robot =
        Isometry3::from(robot_to_chest_centre_right.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_chest_right = kinematics.isometry::<Robot, ChestRight>().inner;
    let imu_adjusted_chest_right_to_robot = Isometry3::from(robot_to_chest_right.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_chest_left_to_robot,
        imu_adjusted_chest_centre_left_to_robot,
        imu_adjusted_chest_centre_right_to_robot,
        imu_adjusted_chest_right_to_robot,
    )
}

// fn robot_to_shoulders(
//     orientation: &RobotOrientation,
//     kinematics: &Kinematics,
// ) -> (
//     Isometry3<f32>,
//     Isometry3<f32>,
//     Isometry3<f32>,
//     Isometry3<f32>,
// ) {
//     let (roll, pitch, _) = orientation.euler_angles();
//
//     let imu_adjusted_left_shoulder_cap_back_to_robot = Isometry3::from(
//         kinematics
//             .left_shoulder_cap_back_to_robot
//             .translation
//             .inverse(),
//     ) * Isometry3::rotation(
//         Vector3::y() * pitch,
//     ) * Isometry3::rotation(Vector3::x() * roll);
//
//     let imu_adjusted_left_shoulder_cap_front_to_robot =
//         Isometry3::from(
//             kinematics
//                 .left_shoulder_cap_front_to_robot
//                 .translation
//                 .inverse(),
//         ) * Isometry3::rotation(Vector3::y() * pitch)
//             * Isometry3::rotation(Vector3::x() * roll);
//
//     let imu_adjusted_right_shoulder_cap_front_to_robot =
//         Isometry3::from(
//             kinematics
//                 .right_shoulder_cap_front_to_robot
//                 .translation
//                 .inverse(),
//         ) * Isometry3::rotation(Vector3::y() * pitch)
//             * Isometry3::rotation(Vector3::x() * roll);
//
//     let imu_adjusted_right_shoulder_cap_back_to_robot =
//         Isometry3::from(
//             kinematics
//                 .right_shoulder_cap_back_to_robot
//                 .translation
//                 .inverse(),
//         ) * Isometry3::rotation(Vector3::y() * pitch)
//             * Isometry3::rotation(Vector3::x() * roll);
//
//     (
//         imu_adjusted_left_shoulder_cap_back_to_robot,
//         imu_adjusted_left_shoulder_cap_front_to_robot,
//         imu_adjusted_right_shoulder_cap_front_to_robot,
//         imu_adjusted_right_shoulder_cap_back_to_robot,
//     )
// }

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
    let camera_pos = pose.as_3d() * matrix.camera_to_ground;

    dbg.log_with_cycle(
        T::make_entity_image_path(""),
        *cycle,
        &rerun::Transform3D::from_translation(Into::<Vec3>::into(camera_pos.translation))
            .with_quaternion(Into::<Quat>::into(camera_pos.rotation)),
    );
}
