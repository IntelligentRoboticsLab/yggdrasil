use nalgebra::{Isometry3, Matrix, Point2, UnitQuaternion, Vector2, Vector3};

use crate::{
    camera::matrix::horizon::Horizon, filter::imu::IMUValues, kinematics::robot_dimensions,
    walk::SwingFoot,
};

const CAMERA_TOP_PITCH_DEGREES: f32 = 1.2;
const CAMERA_BOTTOM_PITCH_DEGREES: f32 = 39.7;

struct CameraConfiguration {
    extrinsic_rotation: Vector3<f32>,
    focal_lengths: Vector2<f32>,
    cc_optical_center: Vector2<f32>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CameraPosition {
    Top,
    Bottom,
}

pub fn robot_to_ground(swing_foot: SwingFoot, imu: IMUValues) -> Isometry3<f32> {
    let roll_pitch = imu.angles;
    let roll = roll_pitch.x;
    let pitch = roll_pitch.y;

    Default::default()
}

pub fn camera_to_head(
    position: CameraPosition,
    extrinsic_rotations: Vector3<f32>,
) -> Isometry3<f32> {
    // first we convert the extrinsic rotations to radians, as configuration is in degrees
    let extrinsic_rotations = extrinsic_rotations.map(|x| x.to_radians());

    // create the rotation quaternion, using the extrinsic rotations as config value.
    let extrinsic_rotation = UnitQuaternion::from_euler_angles(
        extrinsic_rotations.x,
        extrinsic_rotations.y,
        extrinsic_rotations.z,
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

struct CameraMatrix {
    pub camera_to_head: Isometry3<f32>,
    pub robot_to_camera: Isometry3<f32>,
}

impl CameraMatrix {
    pub fn from_normalized_focal_and_center(
        focal_lengths: Vector2<f32>,
        cc_optical_center: Point2<f32>,
        image_size: Vector2<f32>,
        camera_to_head: Isometry3<f32>,
        head_to_robot: Isometry3<f32>,
        robot_to_ground: Isometry3<f32>,
    ) -> Self {
        let camera_to_robot = robot_to_ground * camera_to_head;
        let camera_to_ground = robot_to_ground * camera_to_robot;

        let image_size_diagonal = Matrix::from_diagonal(&image_size);
        let focal_length_scaled = image_size_diagonal * focal_lengths;
        let optical_center_scaled = image_size_diagonal * cc_optical_center;

        let field_of_view = Self::compute_field_of_view(focal_lengths, image_size);

        let horizon = Horizon::from_parameters(
            camera_to_ground,
            focal_length_scaled,
            optical_center_scaled,
            image_size[0],
        );

        Self {
            camera_to_head,
            robot_to_camera: camera_to_robot.inverse(),
        }
    }

    pub fn compute_field_of_view(
        focal_lengths: Vector2<f32>,
        image_size: Vector2<f32>,
    ) -> Vector2<f32> {
        // Ref:  https://www.edmundoptics.eu/knowledge-center/application-notes/imaging/understanding-focal-length-and-field-of-view/
        image_size.zip_map(&focal_lengths, |image_dim, focal_length| -> f32 {
            2.0 * (image_dim * 0.5 / focal_length).atan()
        })
    }
}
