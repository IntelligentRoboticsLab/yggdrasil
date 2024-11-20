use std::ops::Deref;

use bevy::{prelude::*, reflect::List};
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
    #[must_use]
    pub fn is_part_of_body(&self, image_coordinate: Point2<f32>) -> bool {
        self.left_shoulder_cap_point.is_some_and(|shoulder_point| {
            Self::is_part_of_shoulder(shoulder_point, image_coordinate)
        }) || self.right_shoulder_cap_point.is_some_and(|shoulder_point| {
            Self::is_part_of_shoulder(shoulder_point, image_coordinate)
        }) || self
            .chest_point
            .is_some_and(|chest_point| Self::is_part_of_chest(chest_point, image_coordinate))
    }

    fn is_part_of_shoulder(shoulder_point: Point2<f32>, image_coordinate: Point2<f32>) -> bool {
        shoulder_point.x - 20.0 < image_coordinate.x
            && shoulder_point.x + 20.0 > image_coordinate.x
            && shoulder_point.y - 20.0 < image_coordinate.y
            && shoulder_point.y + 20.0 > image_coordinate.y
    }

    fn is_part_of_chest(chest_point: Point2<f32>, image_coordinate: Point2<f32>) -> bool {
        chest_point.x - 40.0 < image_coordinate.x
            && chest_point.x + 40.0 > image_coordinate.x
            && chest_point.y - 20.0 < image_coordinate.y
            && chest_point.y + 40.0 > image_coordinate.y
    }

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
            .add_systems(Update, update_body_contours)
            .add_systems(Update, visualize_body_contour.after(update_body_contours));
    }
}

fn setup_body_contour_visualization<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_component_batches(
        T::make_entity_path("body_contour"),
        true,
        [&rerun::Color::from_rgb(219, 62, 177) as _],
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
    // if bottom_image.is_from_cycle(*current_cycle) {
    //     body_contour.update_toes(&orientation, &kinematics,
    // &bottom_camera_matrix);
    //
    //     body_contour.update_chest(&orientation, &kinematics,
    // &bottom_camera_matrix);
    //
    //     body_contour.update_shoulders(&orientation, &kinematics,
    // &bottom_camera_matrix); }

    // #TODO: for debugging, remove.
    body_contour.chest_point = Some(Point2::new(200.0, 200.0));
    body_contour.left_shoulder_cap_point = Some(Point2::new(160.0, 140.0));
    body_contour.right_shoulder_cap_point = Some(Point2::new(240.0, 140.0));
}

fn visualize_body_contour(
    body_contour: Res<BodyContour>,
    debug_context: DebugContext,
    bottom_image: Res<Image<Bottom>>,
) {
    let mut points = Vec::with_capacity(480 * 720);

    for x in 0..bottom_image.yuyv_image().width() {
        for y in 0..bottom_image.yuyv_image().height() {
            let x = x as f32;
            let y = y as f32;
            if body_contour.is_part_of_body(Point2::new(x, y)) {
                points.push((x, y));
            }
        }
    }

    debug_context.log_with_cycle(
        Bottom::make_entity_path("body_contour"),
        bottom_image.deref().cycle(),
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
