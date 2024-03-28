use miette::{bail, Result};
use nalgebra::{point, vector, Isometry3, Point2, Point3, Vector2, Vector3};

/// A camera matrix that is able to project points.
#[derive(Default, Debug, Clone)]
pub struct CameraMatrix {
    /// The optical center of the camera in the image plane, in pixels.
    pub cc_optical_center: Point2<f32>,
    /// The focal lengths of the camera in pixels.
    pub focal_lengths: Vector2<f32>,
    /// The field of view of the camera in radians.
    pub field_of_view: Vector2<f32>,
    /// The transformation from the camera frame to the head frame.
    pub camera_to_head: Isometry3<f32>,
    /// The transformation from the robot to the camera frame.
    pub robot_to_camera: Isometry3<f32>,
    /// The transformation from camera frame to the ground frame.
    pub camera_to_ground: Isometry3<f32>,
}

impl CameraMatrix {
    pub fn new(
        focal_lengths: Vector2<f32>,
        cc_optical_center: Point2<f32>,
        image_size: Vector2<f32>,
        camera_to_head: Isometry3<f32>,
        head_to_robot: Isometry3<f32>,
        robot_to_ground: Isometry3<f32>,
    ) -> Self {
        let camera_to_robot = head_to_robot * camera_to_head;
        let camera_to_ground = robot_to_ground * camera_to_robot;

        let field_of_view = Self::compute_field_of_view(focal_lengths, image_size);

        Self {
            cc_optical_center,
            focal_lengths,
            field_of_view,
            camera_to_head,
            robot_to_camera: camera_to_robot.inverse(),
            camera_to_ground,
        }
    }

    /// Get a vector pointing from the camera through the given pixel in the image plane.
    ///
    /// This is in the camera's coordinate frame where x is forward, y is left, and z is up.
    pub fn pixel_to_camera(&self, pixel: Point2<f32>) -> Vector3<f32> {
        vector![
            1.0,
            (self.cc_optical_center.x - pixel.x) / self.focal_lengths.x,
            (self.cc_optical_center.y - pixel.y) / self.focal_lengths.y
        ]
    }

    /// Project a pixel to the ground coordinate frame at a given height.
    ///
    /// We assume the ground is at z = 0.0
    ///
    /// # Errors
    /// This fails if the point is above the horizon and cannot be projected to the ground.
    pub fn pixel_to_ground(&self, pixel: Point2<f32>, z: f32) -> Result<Point3<f32>> {
        let camera_ray = self.pixel_to_camera(pixel);
        let camera_ray_over_ground = self.camera_to_ground.rotation * camera_ray;

        if camera_ray_over_ground.z >= 0.0
            || camera_ray_over_ground.x.is_nan()
            || camera_ray_over_ground.y.is_nan()
            || camera_ray_over_ground.z.is_nan()
        {
            bail!("Point is above the horizon and cannot be projected to the ground");
        }

        let distance_to_plane = z - self.camera_to_ground.translation.z;
        let slope = distance_to_plane / camera_ray_over_ground.z;
        let intersection =
            self.camera_to_ground.translation.vector + camera_ray_over_ground * slope;

        Ok(point![intersection.x, intersection.y, z])
    }

    fn compute_field_of_view(focal_lengths: Vector2<f32>, image_dim: Vector2<f32>) -> Vector2<f32> {
        Vector2::new(
            2.0 * (focal_lengths.x / image_dim.x).atan(),
            2.0 * (focal_lengths.y / image_dim.y).atan(),
        )
    }
}
