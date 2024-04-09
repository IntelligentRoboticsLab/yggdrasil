use nalgebra::{point, vector, Point2};

/// A line segment in 2D space.
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl LineSegment {
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { start, end }
    }

    pub fn from_xy(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            start: point![x1, y1],
            end: point![x2, y2],
        }
    }

    pub fn angle(&self) -> f32 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        dy.atan2(dx)
    }

    pub fn is_in_bounding_box(&self, point: Point2<f32>) -> bool {
        let min_x = self.start.x.min(self.end.x);
        let max_x = self.start.x.max(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_y = self.start.y.max(self.end.y);

        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
    }

    pub fn intersection_point(&self, other: &LineSegment) -> Option<Point2<f32>> {
        let delta_x = vector![self.start.x - self.end.x, other.start.x - other.end.x];
        let delta_y = vector![self.start.y - self.end.y, other.start.y - other.end.y];

        let determinant = delta_x.perp(&delta_y);
        if determinant.abs() < std::f32::EPSILON {
            return None;
        }

        let d = vector![
            self.start.x * self.end.y - self.start.y * self.end.x,
            other.start.x * other.end.y - other.start.y * other.end.x,
        ];

        let x = d.perp(&delta_x) / determinant;
        let y = d.perp(&delta_y) / determinant;

        let point = point![x, y];
        if self.is_in_bounding_box(point) && other.is_in_bounding_box(point) {
            Some(point)
        } else {
            None
        }
    }

    pub fn angle_between(&self, other: &LineSegment) -> f32 {
        let angle1 = self.angle();
        let angle2 = other.angle();

        (angle1 - angle2).abs()
    }
}

impl From<&LineSegment> for [(f32, f32); 2] {
    fn from(segment: &LineSegment) -> Self {
        [
            (segment.start.x, segment.start.y),
            (segment.end.x, segment.end.y),
        ]
    }
}
