use bevy::prelude::*;

use nalgebra::{Point2, Vector2};

#[derive(Debug, Clone, Copy, Component)]
pub struct Line2 {
    /// Normal to the line itself
    pub normal: Vector2<f32>,
    /// Distance to the origin
    pub d: f32,
}

#[derive(Debug, Clone, Copy, Component)]
pub struct LineSegment2 {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl LineSegment2 {
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { start, end }
    }

    /// Length of the line segment
    pub fn length(&self) -> f32 {
        (self.end - self.start).norm()
    }

    // Center of the line segment
    pub fn center(&self) -> Point2<f32> {
        (self.start + self.end.coords) / 2.0
    }

    /// Normal vector of the line segment
    pub fn normal(&self) -> Vector2<f32> {
        let dir = self.end - self.start;
        Vector2::new(-dir.y, dir.x).normalize()
    }

    // Samples n points uniformly *in between* the two endpoints (excluding the endpoints themselves).
    pub fn sample_uniform(&self, n: usize) -> impl Iterator<Item = Point2<f32>> + use<'_> {
        (1..=n).map(move |i| {
            let t = i as f32 / (n + 1) as f32;
            self.start + (self.end - self.start) * t
        })
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
