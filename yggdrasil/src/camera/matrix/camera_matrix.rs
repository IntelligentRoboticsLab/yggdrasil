use miette::bail;
use nalgebra::{
    point, vector, Isometry3, Matrix, Point2, Point3, UnitQuaternion, Vector2, Vector3,
};

use crate::{
    camera::matrix::horizon::Horizon,
    debug::DebugContext,
    filter::imu::IMUValues,
    kinematics::{robot_dimensions, RobotKinematics},
    prelude::*,
    walk::{engine::Side, SwingFoot},
};

const CAMERA_TOP_PITCH_DEGREES: f32 = 1.2;
const CAMERA_BOTTOM_PITCH_DEGREES: f32 = 39.7;

struct CameraConfiguration {
    extrinsic_rotation: Vector3<f32>,
    focal_lengths: Vector2<f32>,
    cc_optical_center: Vector2<f32>,
}

#[derive(derive_more::Deref, Default)]
pub struct TopCameraMatrix(pub CameraMatrix);

#[system]
pub fn update_camera_matrix(
    swing_foot: &SwingFoot,
    imu: &IMUValues,
    kinematics: &RobotKinematics,
    dbg: &DebugContext,
    top_camera_matrix: &mut TopCameraMatrix,
) -> Result<()> {
    let image_size = vector![640.0, 480.0];
    let extrinsic_rotation = vector![0.03999999910593033, -3.009999990463257, 1.2999999523162842];

    let focal_lengths = vector![0.95, 1.27];
    let cc_optical_center = point![0.5, 0.5];

    let top_camera_to_head = camera_to_head(CameraPosition::Top, extrinsic_rotation);
    let camera_matrix = CameraMatrix::from_normalized_focal_and_center(
        focal_lengths,
        cc_optical_center,
        image_size,
        top_camera_to_head,
        kinematics.head_to_robot,
        robot_to_ground(swing_foot, imu, kinematics),
    );
    top_camera_matrix.0 = camera_matrix;

    Ok(())
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CameraPosition {
    Top,
    Bottom,
}

pub fn robot_to_ground(
    swing_foot: &SwingFoot,
    imu: &IMUValues,
    kinematics: &RobotKinematics,
) -> Isometry3<f32> {
    let roll_pitch = imu.angles;
    let roll = roll_pitch.x;
    let pitch = roll_pitch.y;

    let left_sole_to_robot = kinematics.left_sole_to_robot;
    let imu_adjusted_robot_to_left_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(left_sole_to_robot.translation.inverse());

    let right_sole_to_robot = kinematics.right_sole_to_robot;
    let imu_adjusted_robot_to_right_sole = Isometry3::rotation(Vector3::y() * pitch)
        * Isometry3::rotation(Vector3::x() * roll)
        * Isometry3::from(right_sole_to_robot.translation.inverse());

    match swing_foot.side {
        Side::Left => imu_adjusted_robot_to_left_sole,
        Side::Right => imu_adjusted_robot_to_right_sole,
    }
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

#[derive(Default)]
pub struct CameraMatrix {
    pub optical_center: Point2<f32>,
    pub camera_to_head: Isometry3<f32>,
    pub robot_to_camera: Isometry3<f32>,
    pub camera_to_ground: Isometry3<f32>,
    pub horizon: Horizon,
    pub focal_length: Vector2<f32>,
    pub field_of_view: Vector2<f32>,
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
        let camera_to_robot = head_to_robot * camera_to_head;
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
            optical_center: cc_optical_center,
            camera_to_head,
            robot_to_camera: camera_to_robot.inverse(),
            camera_to_ground,
            horizon,
            focal_length: focal_length_scaled,
            field_of_view,
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

    pub fn pixel_to_camera(&self, pixel: Point2<f32>) -> Vector3<f32> {
        vector![
            1.0,
            (self.optical_center.x - pixel.x) / self.focal_length.x,
            (self.optical_center.y - pixel.y) / self.focal_length.y
        ]
    }

    pub fn pixel_to_ground(&self, pixel: Point2<f32>, z: f32) -> Result<Point3<f32>> {
        let camera_ray = self.pixel_to_camera(pixel);
        let camera_ray_over_ground = self.camera_to_ground.rotation * camera_ray;

        if camera_ray_over_ground.z >= 0.0
            || camera_ray_over_ground.x.is_nan()
            || camera_ray_over_ground.y.is_nan()
            || camera_ray_over_ground.z.is_nan()
        {
            bail!("Point is over horizon");
        }

        let distance_to_plane = z - self.camera_to_ground.translation.z;
        let slope = distance_to_plane / camera_ray_over_ground.z;
        let intersection_point =
            self.camera_to_ground.translation.vector + camera_ray_over_ground * slope;
        Ok(point![intersection_point.x, intersection_point.y, z])
    }
}
