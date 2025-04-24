use std::ops::Mul;

use bevy::prelude::*;

use nalgebra::{Isometry2, Point2, Vector2, point};
use yggdrasil_config::layout::{FieldConfig, ParallelAxis};

/// A line on the field, which can be a line segment or a circle.
#[derive(Debug, Clone, Copy)]
pub enum FieldLine {
    Segment {
        segment: LineSegment2,
        axis: ParallelAxis,
    },
    Circle(Circle),
}

impl FieldLine {
    /// Projects a point onto the field line and returns the projected point, together with the projection distance.
    #[must_use]
    pub fn project_with_signed_distance(&self, point: Point2<f32>) -> (Point2<f32>, f32) {
        match self {
            FieldLine::Segment { segment, .. } => {
                let (projection, distance) = segment.project_with_signed_distance(point);
                (projection, distance)
            }
            FieldLine::Circle(circle) => {
                let (projection, distance) = circle.project_with_signed_distance(point);
                (projection, distance)
            }
        }
    }

    fn from_segment(segment: LineSegment2) -> Self {
        let delta = segment.end - segment.start;

        let axis = if delta.x.abs() > delta.y.abs() {
            ParallelAxis::X
        } else {
            ParallelAxis::Y
        };

        FieldLine::Segment { segment, axis }
    }
}

impl From<&FieldConfig> for FieldLine {
    /// Returns the field lines described by the field configuration.
    fn from(field_config: &FieldConfig) -> [FieldLine; 18] {
        [
            // Center circle
            FieldLine::Circle(Circle {
                center: point![0.0, 0.0],
                radius: field_config.centre_circle_diameter / 2.0,
            }),
            // Field border
            FieldLine::from_segment(LineSegment2::new(
                point![-field_config.length / 2.0, -field_config.width / 2.0],
                point![field_config.length / 2.0, -field_config.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-field_config.length / 2.0, field_config.width / 2.0],
                point![field_config.length / 2.0, field_config.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-field_config.length / 2.0, -field_config.width / 2.0],
                point![-field_config.length / 2.0, field_config.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![field_config.length / 2.0, -field_config.width / 2.0],
                point![field_config.length / 2.0, field_config.width / 2.0],
            )),
            // Center line
            FieldLine::from_segment(LineSegment2::new(
                point![0.0, -field_config.width / 2.0],
                point![0.0, field_config.width / 2.0],
            )),
            // Goal areas & goal boxes
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0,
                    -field_config.goal_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.goal_area_length,
                    -field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0,
                    field_config.goal_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.goal_area_length,
                    field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0,
                    -field_config.goal_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.goal_area_length,
                    -field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0,
                    field_config.goal_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.goal_area_length,
                    field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0 + field_config.goal_area_length,
                    -field_config.goal_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.goal_area_length,
                    field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0 - field_config.goal_area_length,
                    -field_config.goal_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.goal_area_length,
                    field_config.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0,
                    -field_config.penalty_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.penalty_area_length,
                    -field_config.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0,
                    field_config.penalty_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.penalty_area_length,
                    field_config.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0,
                    -field_config.penalty_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.penalty_area_length,
                    -field_config.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0,
                    field_config.penalty_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.penalty_area_length,
                    field_config.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -field_config.length / 2.0 + field_config.penalty_area_length,
                    -field_config.penalty_area_width / 2.0
                ],
                point![
                    -field_config.length / 2.0 + field_config.penalty_area_length,
                    field_config.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    field_config.length / 2.0 - field_config.penalty_area_length,
                    -field_config.penalty_area_width / 2.0
                ],
                point![
                    field_config.length / 2.0 - field_config.penalty_area_length,
                    field_config.penalty_area_width / 2.0
                ],
            )),
        ]
    }
}

/// A normal form line in 2D space
#[derive(Debug, Clone, Copy, Component, PartialEq)]
pub struct Line2 {
    /// Normal to the line itself
    pub normal: Vector2<f32>,
    /// Distance to the origin
    pub d: f32,
}

impl Line2 {
    /// Creates a line from a normal and a distance to the origin
    #[must_use]
    pub fn new(normal: Vector2<f32>, d: f32) -> Self {
        Self { normal, d }
    }

    /// Signed distance from the line to a point
    #[must_use]
    pub fn signed_distance_to_point(&self, point: Point2<f32>) -> f32 {
        self.normal.dot(&point.coords) - self.d
    }

    /// Distance from the line to a point
    #[must_use]
    pub fn distance_to_point(&self, point: Point2<f32>) -> f32 {
        let signed_distance = self.signed_distance_to_point(point);
        signed_distance.abs()
    }

    /// Projects a point onto the line
    #[must_use]
    pub fn project(&self, point: Point2<f32>) -> Point2<f32> {
        let signed_distance = self.signed_distance_to_point(point);
        point - self.normal * signed_distance
    }
}

/// A line segment in 2D space
#[derive(Debug, Clone, Copy, Component)]
pub struct LineSegment2 {
    /// Start point of the line segment
    pub start: Point2<f32>,
    /// End point of the line segment
    pub end: Point2<f32>,
}

impl LineSegment2 {
    /// Creates a new line segment from two points
    #[must_use]
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { start, end }
    }

    /// Creates a line from the line segment
    #[must_use]
    pub fn to_line(&self) -> Line2 {
        let dir = self.end - self.start;
        let normal = Vector2::new(-dir.y, dir.x).normalize();
        let d = normal.dot(&self.start.coords);
        Line2::new(normal, d)
    }

    /// Flips the start and end points of the line segment
    #[must_use]
    pub fn to_flipped(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
        }
    }

    /// Smallest angle between two line segments
    #[must_use]
    pub fn angle(&self, other: &Self) -> f32 {
        let dir1 = self.end - self.start;
        let dir2 = other.end - other.start;
        dir1.angle(&dir2)
    }

    /// Length of the line segment
    #[must_use]
    pub fn length(&self) -> f32 {
        (self.end - self.start).norm()
    }

    /// Center of the line segment
    #[must_use]
    pub fn center(&self) -> Point2<f32> {
        (self.start + self.end.coords) * 0.5
    }

    /// Normal vector of the line segment
    #[must_use]
    pub fn normal(&self) -> Vector2<f32> {
        let dir = self.end - self.start;
        Vector2::new(-dir.y, dir.x).normalize()
    }

    /// Projects a point onto the line segment
    ///
    /// Returns the projected point and the signed distance to the original point
    #[must_use]
    pub fn project_with_signed_distance(&self, point: Point2<f32>) -> (Point2<f32>, f32) {
        let line = self.to_line();
        let projected = line.project(point);
        if self.contains(projected) {
            let distance = (projected - point).norm();
            (projected, distance)
        // If the projected point is not on the line segment we return the closest endpoint
        } else {
            let start_distance = (point - self.start).norm();
            let end_distance = (point - self.end).norm();
            if start_distance < end_distance {
                (self.start, start_distance)
            } else {
                (self.end, end_distance)
            }
        }
    }

    /// Distance from the line segment to a point
    #[must_use]
    pub fn distance_to_point(&self, point: Point2<f32>) -> f32 {
        let line = self.to_line();
        let projected = line.project(point);
        if self.contains(projected) {
            line.distance_to_point(point)
        } else {
            let start_distance = (point - self.start).norm();
            let end_distance = (point - self.end).norm();
            start_distance.min(end_distance)
        }
    }

    /// Checks if a point is contained within the line segment
    #[must_use]
    pub fn contains(&self, point: Point2<f32>) -> bool {
        let dir = self.end - self.start;
        let start_dir = point - self.start;
        let end_dir = point - self.end;
        dir.dot(&start_dir) >= 0.0 && dir.dot(&end_dir) <= 0.0
    }

    /// Samples n points uniformly *in between* the two endpoints (excluding the endpoints themselves).
    pub fn sample_uniform(&self, n: usize) -> impl Iterator<Item = Point2<f32>> + use<'_> {
        (1..=n).map(move |i| {
            let t = i as f32 / (n + 1) as f32;
            self.start + (self.end - self.start) * t
        })
    }
}

impl Mul<LineSegment2> for Isometry2<f32> {
    type Output = LineSegment2;

    fn mul(self, segment: LineSegment2) -> LineSegment2 {
        LineSegment2 {
            start: self * segment.start,
            end: self * segment.end,
        }
    }
}

impl From<LineSegment2> for [(f32, f32); 2] {
    fn from(segment: LineSegment2) -> Self {
        [
            (segment.start.x, segment.start.y),
            (segment.end.x, segment.end.y),
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Point2<f32>,
    pub radius: f32,
}

impl Circle {
    #[must_use]
    pub fn project_with_signed_distance(&self, point: Point2<f32>) -> (Point2<f32>, f32) {
        let dir = point - self.center;
        let distance = dir.norm();
        let projected = self.center + dir / distance * self.radius;
        (projected, distance - self.radius)
    }
}
