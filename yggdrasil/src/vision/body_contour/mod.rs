use std::ops::Deref;

use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix};
use nalgebra::{Isometry3, Point2, Vector3};

use crate::{
    core::debug::DebugContext,
    kinematics::{
        spaces::{Chest, LeftShoulderCap, LeftToe, RightShoulderCap, RightToe, Robot},
        Kinematics,
    },
    nao::Cycle,
    sensor::orientation::RobotOrientation,
};

use super::camera::Image;

#[derive(Default, Resource)]
pub struct BodyContour {
    left_shoulder_cap_point: Option<Point2<f32>>,
    right_shoulder_cap_point: Option<Point2<f32>>,

    left_toe_point: Option<Point2<f32>>,
    right_toe_point: Option<Point2<f32>>,

    chest_point: Option<Point2<f32>>,
}

impl BodyContour {
    fn update_toes(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let (robot_to_left_toe, robot_to_right_toe) = robot_to_toes(orientation, kinematics);

        self.left_toe_point = matrix
            .ground_to_pixel(
                (robot_to_left_toe.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.right_toe_point = matrix
            .ground_to_pixel(
                (robot_to_right_toe.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
    }

    fn update_chest(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let robot_to_chest = robot_to_chest(orientation, kinematics);

        self.chest_point = matrix
            .ground_to_pixel(
                (robot_to_chest.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
    }

    fn update_shoulders(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let (robot_to_left_shoulder_cap, robot_to_right_shoulder_cap) =
            robot_to_shoulders(orientation, kinematics);

        self.left_shoulder_cap_point = matrix
            .ground_to_pixel(
                (robot_to_left_shoulder_cap.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.right_shoulder_cap_point = matrix
            .ground_to_pixel(
                (robot_to_right_shoulder_cap.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
    }
}

#[derive(Default)]
pub struct BodyContourPlugin;

impl Plugin for BodyContourPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BodyContour>()
            .add_systems(PostStartup, setup_body_contour_visualization::<Bottom>)
            .add_systems(Update, update_body_contours);
        // .add_systems(Update, print_toes::<Bottom>)
        // .add_systems(Update, print_chest::<Bottom>)
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

fn update_body_contours(
    mut body_contour: ResMut<BodyContour>,
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
    bottom_camera_matrix: Res<CameraMatrix<Bottom>>,
    bottom_image: Res<Image<Bottom>>,
    current_cycle: Res<Cycle>,
) {
    if bottom_image.is_from_cycle(*current_cycle) {
        body_contour.update_toes(&orientation, &kinematics, &bottom_camera_matrix);

        body_contour.update_chest(&orientation, &kinematics, &bottom_camera_matrix);

        body_contour.update_shoulders(&orientation, &kinematics, &bottom_camera_matrix);
    }
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
    bottom_image: Res<Image<T>>,
    current_cycle: Res<Cycle>,
) {
    if !bottom_image.is_from_cycle(*current_cycle) {
        return;
    }

    let robot_to_chest = robot_to_chest(&orientation, &kinematics);

    let Ok(chest_point) = matrix.ground_to_pixel(
        (robot_to_chest.inverse() * matrix.robot_to_ground)
            .translation
            .vector
            .into(),
    ) else {
        return;
    };

    debug_context.log_with_cycle(
        T::make_entity_path("body_contour/chest"),
        *current_cycle,
        &rerun::Points2D::new([(chest_point.x, chest_point.y)]),
    );
}

fn print_shoulders<T: CameraLocation>(
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

    let (robot_to_left_shoulder_cap, robot_to_right_shoulder_cap) =
        robot_to_shoulders(&orientation, &kinematics);

    let mut points = Vec::new();
    if let Ok(left_shoulder_cap_point) = matrix.ground_to_pixel(
        (robot_to_left_shoulder_cap.inverse() * matrix.robot_to_ground)
            .translation
            .vector
            .into(),
    ) {
        points.push((left_shoulder_cap_point.x, left_shoulder_cap_point.y));
    }
    if let Ok(right_shoulder_cap_point) = matrix.ground_to_pixel(
        (robot_to_right_shoulder_cap.inverse() * matrix.robot_to_ground)
            .translation
            .vector
            .into(),
    ) {
        points.push((right_shoulder_cap_point.x, right_shoulder_cap_point.y));
    }

    debug_context.log_with_cycle(
        T::make_entity_path("body_contour/shoulders"),
        *current_cycle,
        &rerun::Points2D::new(&points),
    );
}

fn robot_to_toes(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_left_toe = kinematics.isometry::<Robot, LeftToe>().inner;
    let imu_adjusted_robot_to_left_toe = Isometry3::from(robot_to_left_toe.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_right_toe = kinematics.isometry::<Robot, RightToe>().inner;
    let imu_adjusted_robot_to_right_toe = Isometry3::from(robot_to_right_toe.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_left_toe,
        imu_adjusted_robot_to_right_toe,
    )
}

fn robot_to_chest(orientation: &RobotOrientation, kinematics: &Kinematics) -> Isometry3<f32> {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_chest = kinematics.isometry::<Robot, Chest>().inner;
    Isometry3::from(robot_to_chest.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
}

fn robot_to_shoulders(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_left_shoulder_cap = kinematics.isometry::<Robot, LeftShoulderCap>().inner;
    let imu_adjusted_robot_to_left_shoulder_cap =
        Isometry3::from(robot_to_left_shoulder_cap.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_right_shoulder_cap = kinematics.isometry::<Robot, RightShoulderCap>().inner;
    let imu_adjusted_robot_to_right_shoulder_cap =
        Isometry3::from(robot_to_right_shoulder_cap.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_left_shoulder_cap,
        imu_adjusted_robot_to_right_shoulder_cap,
    )
}
