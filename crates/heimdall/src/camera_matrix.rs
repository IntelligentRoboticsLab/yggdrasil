use nalgebra::{Isometry3, Point2, Vector2};

#[derive(Default, Debug, Clone)]
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

        let image_size_diagonal = nalgebra::Matrix::from_diagonal(&image_size);
        // let focal_length_scaled = image_size_diagonal * focal_lengths;
        let optical_center_scaled = image_size_diagonal * cc_optical_center;

        let field_of_view = Self::compute_field_of_view(focal_lengths, image_size);

        let horizon = Horizon::from_parameters(
            camera_to_ground,
            focal_lengths,
            optical_center_scaled,
            image_size[0],
        );

        Self {
            optical_center: cc_optical_center,
            camera_to_head,
            robot_to_camera: camera_to_robot.inverse(),
            camera_to_ground,
            horizon,
            focal_length: focal_lengths,
            field_of_view,
        }
    }

    pub fn compute_field_of_view(
        focal_lengths: Vector2<f32>,
        image_size: Vector2<f32>,
    ) -> Vector2<f32> {
        Vector2::new(
            2.0 * (focal_lengths.x / image_size.x).atan(),
            2.0 * (focal_lengths.y / image_size.y).atan(),
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Horizon {
    pub left_horizon_y: f32,
    pub right_horizon_y: f32,
}

impl Horizon {
    pub fn horizon_y_minimum(&self) -> f32 {
        self.left_horizon_y.min(self.right_horizon_y)
    }

    pub fn y_at_x(&self, x: f32, image_width: f32) -> f32 {
        self.left_horizon_y + x / image_width * (self.right_horizon_y - self.left_horizon_y)
    }

    pub fn from_parameters(
        camera_to_ground: Isometry3<f32>,
        focal_length: Vector2<f32>,
        optical_center: Point2<f32>,
        image_width: f32,
    ) -> Self {
        let rotation_matrix = camera_to_ground.rotation.to_rotation_matrix();
        let horizon_slope_is_infinite = rotation_matrix[(2, 2)] == 0.0;

        if horizon_slope_is_infinite {
            Self::default()
        } else {
            let left_horizon_y = optical_center.y
                + focal_length.y
                    * (rotation_matrix[(2, 0)]
                        + optical_center.x * rotation_matrix[(2, 1)] / focal_length.x)
                    / rotation_matrix[(2, 2)];

            let slope = -focal_length.y * rotation_matrix[(2, 1)]
                / (focal_length.x * rotation_matrix[(2, 2)]);

            // Guesses if image size is in "normalized" (1.0 x 1.0) dimensions
            let adjusted_image_width = if image_width <= 1.0 {
                image_width
            } else {
                image_width - 1.0
            };
            let right_horizon_y = left_horizon_y + (slope * adjusted_image_width);

            Self {
                left_horizon_y,
                right_horizon_y,
            }
        }
    }
}
