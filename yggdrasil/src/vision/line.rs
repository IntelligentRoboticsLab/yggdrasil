use crate::prelude::*;
use heimdall::CameraMatrix;
use miette::Context;
use nalgebra::{point, vector, Point, Point2, Point3};

/// A line segment in 2D space.
#[derive(Debug, Clone, Copy)]
pub struct LineSegment<const DIM: usize> {
    pub start: Point<f32, DIM>,
    pub end: Point<f32, DIM>,
}

pub type LineSegment2 = LineSegment<2>;
pub type LineSegment3 = LineSegment<3>;

impl<const DIM: usize> LineSegment<DIM> {
    pub fn new(start: Point<f32, DIM>, end: Point<f32, DIM>) -> Self {
        Self { start, end }
    }

    pub fn angle(&self) -> f32 {
        self.end.coords.angle(&self.start.coords)
    }

    pub fn angle_between(&self, other: &LineSegment<DIM>) -> f32 {
        let angle1 = self.angle();
        let angle2 = other.angle();

        (angle1 - angle2).abs()
    }
}

impl LineSegment<2> {
    pub fn from_xy(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self {
            start: point![x1, y1],
            end: point![x2, y2],
        }
    }

    pub fn is_in_bounding_box(&self, point: Point2<f32>) -> bool {
        let min_x = self.start.x.min(self.end.x);
        let max_x = self.start.x.max(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_y = self.start.y.max(self.end.y);

        point.x >= min_x && point.x <= max_x && point.y >= min_y && point.y <= max_y
    }

    pub fn intersection_point(&self, other: &LineSegment<2>) -> Option<Point2<f32>> {
        let delta_x = vector![self.start.x - self.end.x, other.start.x - other.end.x];
        let delta_y = vector![self.start.y - self.end.y, other.start.y - other.end.y];

        let determinant = delta_x.perp(&delta_y);
        if determinant.abs() < f32::EPSILON {
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

    pub fn project_to_3d(&self, matrix: &CameraMatrix) -> Result<LineSegment<3>> {
        let start = matrix.pixel_to_ground(self.start, 0.0).with_context(|| {
            format!(
                "Failed to project start point to 3D space: {:?}",
                self.start
            )
        })?;
        let end = matrix
            .pixel_to_ground(self.end, 0.0)
            .with_context(|| format!("Failed to project end point to 3D space: {:?}", self.end))?;

        Ok(LineSegment3::new(start, end))
    }

    pub fn from_projected_xyz(line: &LineSegment<3>, matrix: &CameraMatrix) -> Result<Self> {
        line.project_to_2d(matrix)
    }
}

impl LineSegment<3> {
    pub fn from_xyz(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> Self {
        Self {
            start: point![x1, y1, z1],
            end: point![x2, y2, z2],
        }
    }

    pub fn is_in_bounding_box(&self, point: Point3<f32>) -> bool {
        let min_x = self.start.x.min(self.end.x);
        let max_x = self.start.x.max(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_y = self.start.y.max(self.end.y);
        let min_z = self.start.z.min(self.end.z);
        let max_z = self.start.z.max(self.end.z);

        point.x >= min_x
            && point.x <= max_x
            && point.y >= min_y
            && point.y <= max_y
            && point.z >= min_z
            && point.z <= max_z
    }

    pub fn from_projected_xy(line: &LineSegment<2>, matrix: &CameraMatrix) -> Result<Self> {
        let start = matrix.pixel_to_ground(line.start, 0.0).with_context(|| {
            format!(
                "Failed to project start point to 3D space: {:?}",
                line.start
            )
        })?;
        let end = matrix
            .pixel_to_ground(line.end, 0.0)
            .with_context(|| format!("Failed to project end point to 3D space: {:?}", line.end))?;

        Ok(Self { start, end })
    }

    pub fn project_to_2d(&self, matrix: &CameraMatrix) -> Result<LineSegment<2>> {
        let start = matrix.ground_to_pixel(self.start).with_context(|| {
            format!(
                "Failed to project start point to 2D space: {:?}",
                self.start
            )
        })?;
        let end = matrix
            .ground_to_pixel(self.end)
            .with_context(|| format!("Failed to project end point to 2D space: {:?}", self.end))?;

        Ok(LineSegment::new(start, end))
    }
}

impl From<&LineSegment<2>> for [(f32, f32); 2] {
    fn from(segment: &LineSegment<2>) -> Self {
        [
            (segment.start.x, segment.start.y),
            (segment.end.x, segment.end.y),
        ]
    }
}

impl From<LineSegment<2>> for [(f32, f32); 2] {
    fn from(segment: LineSegment<2>) -> Self {
        [
            (segment.start.x, segment.start.y),
            (segment.end.x, segment.end.y),
        ]
    }
}

impl From<&LineSegment<3>> for [(f32, f32, f32); 2] {
    fn from(segment: &LineSegment<3>) -> Self {
        [
            (segment.start.x, segment.start.y, segment.start.z),
            (segment.end.x, segment.end.y, segment.end.z),
        ]
    }
}
impl From<LineSegment<3>> for [(f32, f32, f32); 2] {
    fn from(segment: LineSegment<3>) -> Self {
        [
            (segment.start.x, segment.start.y, segment.start.z),
            (segment.end.x, segment.end.y, segment.end.z),
        ]
    }
}
