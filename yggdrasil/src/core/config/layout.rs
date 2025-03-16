use bevy::prelude::*;
use nalgebra::point;
use nalgebra::Point2;
use nalgebra::Vector2;
use odal::Config;
use serde::{Deserialize, Serialize};

use crate::vision::line_detection::line::Circle;
use crate::vision::line_detection::line::LineSegment2;

/// Config that contains information about the layout of the field.
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LayoutConfig {
    pub field: FieldConfig,
}

/// Config that contains information about the field dimensions.
/// A schematic overview is given below:
///
/// ```markdown
/// .---------------------------------------------------------------------------.
/// |                                                                           |
/// |                                                                           |
/// |        <----------------------------A--------------------------->         |
/// |      .------------------------------------------------------------.       |
/// |    ^ |                              |                             |       |
/// |    | | <--G-->                      |                             |       |
/// |    | |----------.                   |                  .----------|       |
/// |    | |          | ^                 |                  |          |       |
/// |    | |<E>       | |                 |                  |          |       |
/// |    | |---.      | |                 |                  |      .---|       |
/// |    | |   | ^    | |               -----                |      |   |       |
/// |    | |   | |    | |              /  |  \               |      |   |       |
/// |    B |   | F 0  | H             |<--J-->|              |  0<--I-->|       |
/// |    | |   | |    | |              \  |  /               |      |   |       |
/// |    | |   | v    | |               -----                |      |   |       |
/// |    | |---.      | |                 |                  |      .---|       |
/// |    | |          | |                 |                  |          |       |
/// |    | |          | v                 |                  |          |       |
/// |    | |----------.                   |                  .----------|       |
/// |    | |                              |                             |       |
/// |    v |                              |                             |<--K-->|
/// |      .------------------------------------------------------------.       |
/// |                                                                 ^         |
/// |                                                                 K         |
/// |                                                                 v         |
/// .---------------------------------------------------------------------------.
/// ```
///
/// Here it is assumed the centre point as coordinates (0, 0).
/// The x axis points towards the opponents' goal and runs parallel with A.
/// The y axis points towards the top and runs parallel with B.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    /// Field length in metres (A)
    pub length: f32,
    /// Field width in metres (B)
    pub width: f32,
    /// Width of lines on the field (|)
    pub line_width: f32,
    /// Size of the penalty mark (0)
    pub penalty_mark_size: f32,
    /// Length of the goal area (E)
    pub goal_area_length: f32,
    /// Width of the goal area (F)
    pub goal_area_width: f32,
    /// Length of the penalty area (G)
    pub penalty_area_length: f32,
    /// Width of the penalty area (H)
    pub penalty_area_width: f32,
    /// Distance to the penalty mark from the side of the field (I)
    pub penalty_mark_distance: f32,
    /// Diameter of the centre circle (J)
    pub centre_circle_diameter: f32,
    /// Width of the border strip (K)
    pub border_strip_width: f32,
}

/// A line on the field, which can be a line segment or a circle.
#[derive(Debug, Clone, Copy)]
pub enum FieldLine {
    Segment(LineSegment2),
    Circle(Circle),
}

impl FieldLine {
    /// Projects a point onto the field line and returns the projected point, together with the projection distance.
    #[must_use]
    pub fn project_with_distance(&self, point: Point2<f32>) -> (Point2<f32>, f32) {
        match self {
            FieldLine::Segment(segment) => {
                let (projection, distance) = segment.project_with_distance(point);
                (projection, distance)
            }
            FieldLine::Circle(circle) => {
                let (projection, distance) = circle.project_with_distance(point);
                (projection, distance)
            }
        }
    }
}

impl FieldConfig {
    /// Returns the diagonal of the field.
    #[must_use]
    pub fn diagonal(&self) -> Vector2<f32> {
        Vector2::new(self.length, self.width)
    }

    /// Returns if the point is in the field.
    #[must_use]
    pub fn in_field(&self, point: Point2<f32>) -> bool {
        self.in_field_with_margin(point, 0.0)
    }

    /// Returns if the point is in the field with a margin/slack distance.
    #[must_use]
    pub fn in_field_with_margin(&self, point: Point2<f32>, margin: f32) -> bool {
        point.x.abs() < self.length / 2.0 + margin && point.y.abs() < self.width / 2.0 + margin
    }

    /// Returns the field lines described by the field configuration.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn field_lines(&self) -> [FieldLine; 18] {
        [
            // Center circle
            FieldLine::Circle(Circle {
                center: point![0.0, 0.0],
                radius: self.centre_circle_diameter / 2.0,
            }),
            // Field border
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, -self.width / 2.0],
                point![self.length / 2.0, -self.width / 2.0],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, self.width / 2.0],
                point![self.length / 2.0, self.width / 2.0],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, -self.width / 2.0],
                point![-self.length / 2.0, self.width / 2.0],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![self.length / 2.0, -self.width / 2.0],
                point![self.length / 2.0, self.width / 2.0],
            )),
            // Center line
            FieldLine::Segment(LineSegment2::new(
                point![0.0, -self.width / 2.0],
                point![0.0, self.width / 2.0],
            )),
            // Goal areas & goal boxes
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, -self.goal_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, self.goal_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![self.length / 2.0, -self.goal_area_width / 2.0],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![self.length / 2.0, self.goal_area_width / 2.0],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![
                    self.length / 2.0 - self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, -self.penalty_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![-self.length / 2.0, self.penalty_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![self.length / 2.0, -self.penalty_area_width / 2.0],
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![self.length / 2.0, self.penalty_area_width / 2.0],
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::Segment(LineSegment2::new(
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
        ]
    }
}

impl Config for LayoutConfig {
    const PATH: &'static str = "layout.toml";
}
