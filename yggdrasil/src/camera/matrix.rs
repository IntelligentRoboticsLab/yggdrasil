use nalgebra::{Isometry3, UnitQuaternion, Vector2, Vector3};

use crate::kinematics::robot_dimensions;

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
    pub fn new(
        focal_lengths: Vector2<f32>,
        cc_optical_center: Vector2<f32>,
        image_size: Vector2<f32>,
        camera_to_head: Isometry3<f32>,
        head_to_robot: Isometry3<f32>,
        robot_to_gorund: Isometry3<f32>,
    ) -> Self {
        todo!()
    }
}
