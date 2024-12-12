use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix};
use nalgebra::{Isometry3, Point2, Vector3};

use crate::{
    core::debug::DebugContext, kinematics::prelude::*, nao::Cycle,
    sensor::orientation::RobotOrientation,
};

use super::camera::Image;

//#[derive(Default, Resource, Debug, Deref)]
//struct ChestPoints(Vec<Point2<f32>>);
type ChestPoints = Vec<Point2<f32>>;

struct ShoulderCapPoints {
    front: Point2<f32>,
    back: Point2<f32>,
}

#[derive(Default, Resource)]
struct BodyContour {
    left_shoulder_cap_point: Option<ShoulderCapPoints>,
    right_shoulder_cap_point: Option<ShoulderCapPoints>,

    left_toe_point: Option<Point2<f32>>,
    right_toe_point: Option<Point2<f32>>,

    chest_points: ChestPoints,

    left_thigh_point: Option<Point2<f32>>,
    right_thigh_point: Option<Point2<f32>>,

    left_tibia_point: Option<Point2<f32>>,
    right_tibia_point: Option<Point2<f32>>,
}

impl BodyContour {
    #[must_use]
    pub fn is_part_of_body(&self, image_coordinate: Point2<f32>) -> bool {
        self.left_shoulder_cap_point
            .as_ref()
            .is_some_and(|shoulder_point| {
                Self::is_part_of_shoulder(shoulder_point, image_coordinate)
            })
            || self
                .right_shoulder_cap_point
                .as_ref()
                .is_some_and(|shoulder_point| {
                    Self::is_part_of_shoulder(shoulder_point, image_coordinate)
                })
            || Self::is_part_of_chest(&self.chest_points, image_coordinate)
            || self
                .left_thigh_point
                .is_some_and(|thigh_point| Self::is_part_of_thigh(thigh_point, image_coordinate))
            || self
                .right_thigh_point
                .is_some_and(|thigh_point| Self::is_part_of_thigh(thigh_point, image_coordinate))
            || self
                .left_tibia_point
                .is_some_and(|tibia_point| Self::is_part_of_tibia(tibia_point, image_coordinate))
            || self
                .right_tibia_point
                .is_some_and(|tibia_point| Self::is_part_of_tibia(tibia_point, image_coordinate))
    }

    fn is_part_of_shoulder(
        shoulder_point: &ShoulderCapPoints,
        image_coordinate: Point2<f32>,
    ) -> bool {
        let (left_point, right_point) = match (shoulder_point.back, shoulder_point.front) {
            (left_point, right_point) if left_point.x <= right_point.x => (left_point, right_point),
            (right_point, left_point) => (left_point, right_point),
        };

        image_coordinate.x > left_point.x && image_coordinate.x < right_point.x
    }

    fn is_part_of_chest(chest_points: &ChestPoints, image_coordinate: Point2<f32>) -> bool {
        if image_coordinate.x <= chest_points.first().unwrap().x
            || image_coordinate.x >= chest_points.last().unwrap().x
        {
            return false;
        }

        for (left_point, right_point) in chest_points.iter().zip(chest_points.iter().skip(1)) {
            if image_coordinate.x < right_point.x {
                let a = (right_point.y - left_point.y) / (right_point.x - left_point.x);
                return left_point.y + (image_coordinate.x - left_point.x) * a
                    <= image_coordinate.y;
            }
        }

        unreachable!();
    }

    // # TODO: This might be too simple for a body part that's not static.
    fn is_part_of_thigh(thigh_point: Point2<f32>, image_coordinate: Point2<f32>) -> bool {
        thigh_point.x - 60.0 < image_coordinate.x
            && thigh_point.x + 60.0 > image_coordinate.x
            && thigh_point.y - 100.0 < image_coordinate.y
            && thigh_point.y + 80.0 > image_coordinate.y
    }

    // # TODO: This might be too simple for a body part that's not static.
    fn is_part_of_tibia(tibia_point: Point2<f32>, image_coordinate: Point2<f32>) -> bool {
        tibia_point.x - 40.0 < image_coordinate.x
            && tibia_point.x + 80.0 > image_coordinate.x
            && tibia_point.y - 0.0 < image_coordinate.y
            && tibia_point.y + 80.0 > image_coordinate.y
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
        let (robot_to_chest_left, robot_to_chest, robot_to_chest_right) =
            robot_to_chest(orientation, kinematics);

        self.chest_points.clear();
        let chest_points = &mut self.chest_points;

        if let Ok(chest_left_point) = matrix.ground_to_pixel(
            (robot_to_chest_left.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ) {
            chest_points.push(chest_left_point);
        };

        if let Ok(chest_point) = matrix.ground_to_pixel(
            (robot_to_chest.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ) {
            chest_points.push(chest_point);
        };

        if let Ok(chest_right_point) = matrix.ground_to_pixel(
            (robot_to_chest_right.inverse() * matrix.robot_to_ground)
                .translation
                .vector
                .into(),
        ) {
            chest_points.push(chest_right_point);
        };
    }

    fn update_shoulders(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let (
            (robot_to_left_shoulder_cap_front, robot_to_left_shoulder_cap_back),
            (robot_to_right_shoulder_cap_front, robot_to_right_shoulder_cap_back),
        ) = robot_to_shoulders(orientation, kinematics);

        eprintln!("LEFT FRONT: {robot_to_left_shoulder_cap_front}");
        eprintln!("LEFT BACK: {robot_to_left_shoulder_cap_back}");

        if let (Ok(left_cap_point_front), Ok(left_cap_point_back)) = (
            matrix.ground_to_pixel(
                (robot_to_left_shoulder_cap_front.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            ),
            matrix.ground_to_pixel(
                (robot_to_left_shoulder_cap_back.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            ),
        ) {
            eprintln!("HERE HERE HERE");
            self.left_shoulder_cap_point = Some(ShoulderCapPoints {
                front: left_cap_point_front,
                back: left_cap_point_back,
            });
        } else {
            self.left_shoulder_cap_point = None;
        }

        if let (Ok(right_cap_point_front), Ok(right_cap_point_back)) = (
            matrix.ground_to_pixel(
                (robot_to_right_shoulder_cap_front.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            ),
            matrix.ground_to_pixel(
                (robot_to_right_shoulder_cap_back.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            ),
        ) {
            self.left_shoulder_cap_point = Some(ShoulderCapPoints {
                front: right_cap_point_front,
                back: right_cap_point_back,
            });
        } else {
            self.left_shoulder_cap_point = None;
        }
    }

    fn update_thighs(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let (robot_to_left_thigh, robot_to_right_thigh) = robot_to_thighs(orientation, kinematics);

        self.left_thigh_point = matrix
            .ground_to_pixel(
                (robot_to_left_thigh.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.right_thigh_point = matrix
            .ground_to_pixel(
                (robot_to_right_thigh.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
    }

    fn update_tibias(
        &mut self,
        orientation: &RobotOrientation,
        kinematics: &Kinematics,
        matrix: &CameraMatrix<Bottom>,
    ) {
        let (robot_to_left_tibia, robot_to_right_tibia) = robot_to_tibias(orientation, kinematics);

        self.left_tibia_point = matrix
            .ground_to_pixel(
                (robot_to_left_tibia.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.right_tibia_point = matrix
            .ground_to_pixel(
                (robot_to_right_tibia.inverse() * matrix.robot_to_ground)
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
            .add_systems(
                Update,
                update_body_contours.after(super::camera::matrix::update_camera_matrix::<Bottom>),
            )
            .add_systems(Update, visualize_body_contour.after(update_body_contours));
    }
}

fn setup_body_contour_visualization<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_component_batches(
        T::make_entity_path("image/body_contour"),
        true,
        [
            &rerun::Color::from_rgb(219, 62, 177) as _,
            // &rerun::Radius::new_ui_points(14.0) as _,
            &rerun::Radius::new_ui_points(4.0) as _,
        ],
    );

    dbg.log_component_batches(
        T::make_entity_path("image/body_contour/chests"),
        true,
        [
            &rerun::Color::from_rgb(167, 82, 64) as _,
            // &rerun::Radius::new_ui_points(14.0) as _,
            &rerun::Radius::new_ui_points(4.0) as _,
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
    if !bottom_image.is_from_cycle(*current_cycle) {
        return;
    }

    body_contour.update_toes(&orientation, &kinematics, &bottom_camera_matrix);
    body_contour.update_chest(&orientation, &kinematics, &bottom_camera_matrix);
    body_contour.update_shoulders(&orientation, &kinematics, &bottom_camera_matrix);
    body_contour.update_thighs(&orientation, &kinematics, &bottom_camera_matrix);
    body_contour.update_tibias(&orientation, &kinematics, &bottom_camera_matrix);
}

fn visualize_body_contour(
    body_contour: Res<BodyContour>,
    debug_context: DebugContext,
    bottom_image: Res<Image<Bottom>>,
    current_cycle: Res<Cycle>,
) {
    // # TODO: This function is very slow.
    // It's probably better to let `BodyContour` return the points that should be
    // visualized, instead of iterating over all points.
    // let mut points = Vec::with_capacity(480 * 720);
    let mut points = Vec::new();

    for x in (0..bottom_image.yuyv_image().width()).step_by(10) {
        for y in (0..bottom_image.yuyv_image().height()).step_by(10) {
            let x = x as f32;
            let y = y as f32;
            if body_contour.is_part_of_body(Point2::new(x, y)) {
                points.push((x, y));
            }
        }
    }

    debug_context.log_with_cycle(
        Bottom::make_entity_path("image/body_contour/chests"),
        *current_cycle,
        &rerun::Points2D::new(&points),
    );

    debug_context.log_with_cycle(
        Bottom::make_entity_path("image/body_contour"),
        *current_cycle,
        &rerun::Points2D::new(
            body_contour
                .chest_points
                .iter()
                .map(|point| (point.x, point.y)),
        ),
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

fn robot_to_chest(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_chest_left = kinematics.isometry::<Robot, ChestLeft>().inner;
    let imu_adjusted_robot_to_chest_left = Isometry3::from(robot_to_chest_left.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_chest = kinematics.isometry::<Robot, Chest>().inner;
    let imu_adjusted_robot_to_chest = Isometry3::from(robot_to_chest.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_chest_right = kinematics.isometry::<Robot, ChestRight>().inner;
    let imu_adjusted_robot_to_chest_right = Isometry3::from(robot_to_chest_right.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_chest_left,
        imu_adjusted_robot_to_chest,
        imu_adjusted_robot_to_chest_right,
    )
}

fn robot_to_shoulders(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (
    (Isometry3<f32>, Isometry3<f32>),
    (Isometry3<f32>, Isometry3<f32>),
) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_left_shoulder_cap_front =
        kinematics.isometry::<Robot, LeftShoulderCapFront>().inner;
    let imu_adjusted_robot_to_left_shoulder_cap_front =
        Isometry3::from(robot_to_left_shoulder_cap_front.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);
    let robot_to_left_shoulder_cap_back = kinematics.isometry::<Robot, LeftShoulderCapBack>().inner;
    let imu_adjusted_robot_to_left_shoulder_cap_back =
        Isometry3::from(robot_to_left_shoulder_cap_back.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_right_shoulder_cap_front =
        kinematics.isometry::<Robot, RightShoulderCapFront>().inner;
    let imu_adjusted_robot_to_right_shoulder_cap_front =
        Isometry3::from(robot_to_right_shoulder_cap_front.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);
    let robot_to_right_shoulder_cap_back =
        kinematics.isometry::<Robot, RightShoulderCapBack>().inner;
    let imu_adjusted_robot_to_right_shoulder_cap_back =
        Isometry3::from(robot_to_right_shoulder_cap_back.translation)
            * Isometry3::rotation(Vector3::y() * pitch)
            * Isometry3::rotation(Vector3::x() * roll);

    (
        (
            imu_adjusted_robot_to_left_shoulder_cap_front,
            imu_adjusted_robot_to_left_shoulder_cap_back,
        ),
        (
            imu_adjusted_robot_to_right_shoulder_cap_front,
            imu_adjusted_robot_to_right_shoulder_cap_back,
        ),
    )
}

fn robot_to_thighs(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_left_thigh = kinematics.isometry::<Robot, LeftThigh>().inner;
    let imu_adjusted_robot_to_left_thigh = Isometry3::from(robot_to_left_thigh.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_right_thigh = kinematics.isometry::<Robot, RightThigh>().inner;
    let imu_adjusted_robot_to_right_thigh = Isometry3::from(robot_to_right_thigh.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_left_thigh,
        imu_adjusted_robot_to_right_thigh,
    )
}

fn robot_to_tibias(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let (roll, pitch, _) = orientation.euler_angles();

    let robot_to_left_tibia = kinematics.isometry::<Robot, LeftTibia>().inner;
    let imu_adjusted_robot_to_left_tibia = Isometry3::from(robot_to_left_tibia.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    let robot_to_right_tibia = kinematics.isometry::<Robot, RightTibia>().inner;
    let imu_adjusted_robot_to_right_tibia = Isometry3::from(robot_to_right_tibia.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll);

    (
        imu_adjusted_robot_to_left_tibia,
        imu_adjusted_robot_to_right_tibia,
    )
}
