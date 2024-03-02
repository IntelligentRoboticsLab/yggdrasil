use nalgebra::{Isometry3, Point2, Vector2};

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
                    / (rotation_matrix[(2, 2)]
                        + optical_center.x * rotation_matrix[(2, 2)] / focal_length.x);

            let slope = -focal_length.y * rotation_matrix[(2, 1)]
                / (focal_length.x * rotation_matrix[(2, 2)]);

            let right_horizon_y = left_horizon_y + slope * image_width;

            Self {
                left_horizon_y,
                right_horizon_y,
            }
        }
    }
}
