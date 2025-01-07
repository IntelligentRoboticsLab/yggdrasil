use std::ops::Mul;

use bevy::prelude::*;

use nalgebra::{Isometry2, Point2, Vector2};

/// A line in 2D space
#[derive(Debug, Clone, Copy, Component, PartialEq)]
pub struct Line2 {
    /// Normal to the line itself
    pub normal: Vector2<f32>,
    /// Distance to the origin
    pub d: f32,
}

/// A line segment in 2D space
#[derive(Debug, Clone, Copy, Component)]
pub struct LineSegment2 {
    /// Start point of the line segment
    pub start: Point2<f32>,
    /// End point of the line segment
    pub end: Point2<f32>,
}

impl Line2 {
    /// Creates a line from a normal and a distance to the origin
    #[must_use]
    pub fn new(normal: Vector2<f32>, d: f32) -> Self {
        Self { normal, d }
    }

    /// Distance from the line to a point
    #[must_use]
    pub fn distance_to_point(&self, point: Point2<f32>) -> f32 {
        let signed_distance = self.normal.dot(&point.coords) - self.d;
        signed_distance.abs()
    }
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
