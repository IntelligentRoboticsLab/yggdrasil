use crate::core::debug::debug_system::{DebugAppExt, SystemToggle};
use bevy::prelude::*;
use heimdall::{Bottom, CameraLocation, CameraMatrix};
use nalgebra::{Isometry3, Point2, Translation3, Vector3};

use crate::{
    core::debug::DebugContext, kinematics::prelude::*, nao::Cycle,
    sensor::orientation::RobotOrientation,
};

use super::camera::Image;

const VISUALIZE_DOT_INTERVAL: usize = 10;

type ChestPoints = Vec<Point2<f32>>;

#[derive(Default, Clone)]
struct ShoulderCapPoints {
    front: Option<Point2<f32>>,
    back: Option<Point2<f32>>,
}

#[derive(Default, Resource, Clone)]
pub struct BodyContour {
    left_shoulder_cap_points: ShoulderCapPoints,
    right_shoulder_cap_points: ShoulderCapPoints,

    chest_points: ChestPoints,

    left_thigh_point: Option<Point2<f32>>,
    right_thigh_point: Option<Point2<f32>>,

    left_tibia_point: Option<Point2<f32>>,
    right_tibia_point: Option<Point2<f32>>,
}

impl BodyContour {
    #[must_use]
    pub fn is_part_of_body(&self, image_coordinate: Point2<f32>) -> bool {
        self.is_part_of_left_shoulder(image_coordinate)
            || self.is_part_of_right_shoulder(image_coordinate)
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

    fn is_part_of_left_shoulder(&self, image_coordinate: Point2<f32>) -> bool {
        self.left_shoulder_cap_points
            .front
            .is_some_and(|point| point.x > image_coordinate.x)
            && self
                .left_shoulder_cap_points
                .back
                .is_none_or(|point| point.x < image_coordinate.x)
    }

    fn is_part_of_right_shoulder(&self, image_coordinate: Point2<f32>) -> bool {
        self.right_shoulder_cap_points
            .front
            .is_some_and(|point| point.x < image_coordinate.x)
            && self
                .right_shoulder_cap_points
                .back
                .is_none_or(|point| point.x > image_coordinate.x)
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

    fn update_shoulders(&mut self, orientation: &RobotOrientation, matrix: &CameraMatrix<Bottom>) {
        let (
            (robot_to_left_shoulder_cap_front, robot_to_left_shoulder_cap_back),
            (robot_to_right_shoulder_cap_front, robot_to_right_shoulder_cap_back),
        ) = robot_to_shoulders(orientation);

        self.left_shoulder_cap_points.front = matrix
            .ground_to_pixel(
                (robot_to_left_shoulder_cap_front.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.left_shoulder_cap_points.back = matrix
            .ground_to_pixel(
                (robot_to_left_shoulder_cap_back.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();

        self.right_shoulder_cap_points.front = matrix
            .ground_to_pixel(
                (robot_to_right_shoulder_cap_front.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
        self.right_shoulder_cap_points.back = matrix
            .ground_to_pixel(
                (robot_to_right_shoulder_cap_back.inverse() * matrix.robot_to_ground)
                    .translation
                    .vector
                    .into(),
            )
            .ok();
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
                update_body_contours
                    .after(super::camera::fetch_latest_frame::<Bottom>)
                    .run_if(resource_changed::<Image<Bottom>>),
            )
            .add_named_debug_systems(
                PostUpdate,
                visualize_body_contour.run_if(resource_changed::<BodyContour>),
                "visualize body contour",
                SystemToggle::Disable,
            );
    }
}

fn setup_body_contour_visualization<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_component_batches(
        T::make_entity_path("image/body_contour"),
        true,
        [
            &rerun::Color::from_rgb(167, 82, 64) as _,
            &rerun::Radius::new_ui_points(4.0) as _,
        ],
    );
}

fn update_body_contours(
    mut body_contour: ResMut<BodyContour>,
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
    bottom_camera_matrix: Res<CameraMatrix<Bottom>>,
) {
    body_contour.update_chest(&orientation, &kinematics, &bottom_camera_matrix);
    body_contour.update_shoulders(&orientation, &bottom_camera_matrix);
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
    let mut points = Vec::new();
    for x in (0..bottom_image.yuyv_image().width()).step_by(VISUALIZE_DOT_INTERVAL) {
        for y in (0..bottom_image.yuyv_image().height()).step_by(VISUALIZE_DOT_INTERVAL) {
            let x = x as f32;
            let y = y as f32;
            if body_contour.is_part_of_body(Point2::new(x, y)) {
                points.push((x, y));
            }
        }
    }

    debug_context.log_with_cycle(
        Bottom::make_entity_path("image/body_contour"),
        *current_cycle,
        &rerun::Points2D::new(&points),
    );
}

fn adjust_for_imu(orientation: &RobotOrientation, isometry: Isometry3<f32>) -> Isometry3<f32> {
    let (roll, pitch, _) = orientation.euler_angles();

    Isometry3::from(isometry.translation)
        * Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
}

fn robot_to_chest(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>, Isometry3<f32>) {
    let robot_to_chest = kinematics.isometry::<Robot, Chest>().inner;

    let robot_to_chest_left = Isometry3::from(Translation3::from(
        robot_to_chest.translation.vector + -CHEST_TO_CHEST_LEFT,
    ));
    let robot_to_chest_right = Isometry3::from(Translation3::from(
        robot_to_chest.translation.vector + -CHEST_TO_CHEST_RIGHT,
    ));

    (
        adjust_for_imu(orientation, robot_to_chest_left),
        adjust_for_imu(orientation, robot_to_chest),
        adjust_for_imu(orientation, robot_to_chest_right),
    )
}

fn robot_to_shoulders(
    orientation: &RobotOrientation,
) -> (
    (Isometry3<f32>, Isometry3<f32>),
    (Isometry3<f32>, Isometry3<f32>),
) {
    let robot_to_left_shoulder_cap =
        Isometry3::from(Translation3::from(-ROBOT_TO_LEFT_SHOULDER_CAP));
    let robot_to_left_shoulder_cap_front = Isometry3::from(Translation3::from(
        robot_to_left_shoulder_cap.translation.vector + -SHOULDER_CAP_TO_SHOULDER_CAP_FRONT,
    ));
    let robot_to_left_shoulder_cap_back = Isometry3::from(Translation3::from(
        robot_to_left_shoulder_cap.translation.vector + -SHOULDER_CAP_TO_SHOULDER_CAP_BACK,
    ));

    let robot_to_right_shoulder_cap =
        Isometry3::from(Translation3::from(-ROBOT_TO_RIGHT_SHOULDER_CAP));
    let robot_to_right_shoulder_cap_front = Isometry3::from(Translation3::from(
        robot_to_right_shoulder_cap.translation.vector + -SHOULDER_CAP_TO_SHOULDER_CAP_FRONT,
    ));
    let robot_to_right_shoulder_cap_back = Isometry3::from(Translation3::from(
        robot_to_right_shoulder_cap.translation.vector + -SHOULDER_CAP_TO_SHOULDER_CAP_BACK,
    ));

    (
        (
            adjust_for_imu(orientation, robot_to_left_shoulder_cap_front),
            adjust_for_imu(orientation, robot_to_left_shoulder_cap_back),
        ),
        (
            adjust_for_imu(orientation, robot_to_right_shoulder_cap_front),
            adjust_for_imu(orientation, robot_to_right_shoulder_cap_back),
        ),
    )
}

fn robot_to_thighs(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let robot_to_left_thigh = kinematics.isometry::<Robot, LeftThigh>().inner;
    let robot_to_right_thigh = kinematics.isometry::<Robot, RightThigh>().inner;

    (
        adjust_for_imu(orientation, robot_to_left_thigh),
        adjust_for_imu(orientation, robot_to_right_thigh),
    )
}

fn robot_to_tibias(
    orientation: &RobotOrientation,
    kinematics: &Kinematics,
) -> (Isometry3<f32>, Isometry3<f32>) {
    let robot_to_left_tibia = kinematics.isometry::<Robot, LeftTibia>().inner;
    let robot_to_right_tibia = kinematics.isometry::<Robot, RightTibia>().inner;

    (
        adjust_for_imu(orientation, robot_to_left_tibia),
        adjust_for_imu(orientation, robot_to_right_tibia),
    )
}
