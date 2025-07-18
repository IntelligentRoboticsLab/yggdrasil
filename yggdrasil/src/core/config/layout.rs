use bevy::prelude::*;
use nalgebra::Point2;
use nalgebra::point;
use std::ops::Index;

use nalgebra::Isometry2;
use nalgebra::Vector2;
use odal::Config;
use serde::{Deserialize, Serialize};

use crate::vision::line_detection::line::Circle;
use crate::vision::line_detection::line::LineSegment2;

mod isometry_with_angle {
    use nalgebra::{Isometry, Isometry2, UnitComplex};

    use serde::{Deserialize, Deserializer};

    pub fn deserialize_vec<'de, D>(deserializer: D) -> Result<Vec<Isometry2<f32>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let isometries = Vec::<Isometry<f32, f32, 2>>::deserialize(deserializer)?;

        Ok(isometries
            .into_iter()
            .map(|isometry| {
                Isometry::from_parts(
                    isometry.translation,
                    UnitComplex::new(isometry.rotation.to_radians()),
                )
            })
            .collect())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Isometry2<f32>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let isometry = Isometry::<f32, f32, 2>::deserialize(deserializer)?;

        Ok(Isometry::from_parts(
            isometry.translation,
            UnitComplex::new(isometry.rotation.to_radians()),
        ))
    }
}

/// Config that contains information about the layout of the field and
/// robot positions.
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LayoutConfig {
    pub field: FieldConfig,
    pub initial_positions: FieldPositionsConfig,
    pub set_positions: FieldPositionsConfig,

    #[serde(deserialize_with = "isometry_with_angle::deserialize_vec")]
    pub penalty_positions: Vec<Isometry2<f32>>,
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

/// The axis along which the field line is parallel.
#[derive(Debug, Clone, Copy)]
pub enum ParallelAxis {
    X,
    Y,
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
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, -self.width / 2.0],
                point![self.length / 2.0, -self.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, self.width / 2.0],
                point![self.length / 2.0, self.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, -self.width / 2.0],
                point![-self.length / 2.0, self.width / 2.0],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![self.length / 2.0, -self.width / 2.0],
                point![self.length / 2.0, self.width / 2.0],
            )),
            // Center line
            FieldLine::from_segment(LineSegment2::new(
                point![0.0, -self.width / 2.0],
                point![0.0, self.width / 2.0],
            )),
            // Goal areas & goal boxes
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, -self.goal_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, self.goal_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![self.length / 2.0, -self.goal_area_width / 2.0],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![self.length / 2.0, self.goal_area_width / 2.0],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
                point![
                    -self.length / 2.0 + self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    self.length / 2.0 - self.goal_area_length,
                    -self.goal_area_width / 2.0
                ],
                point![
                    self.length / 2.0 - self.goal_area_length,
                    self.goal_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, -self.penalty_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![-self.length / 2.0, self.penalty_area_width / 2.0],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![self.length / 2.0, -self.penalty_area_width / 2.0],
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![self.length / 2.0, self.penalty_area_width / 2.0],
                point![
                    self.length / 2.0 - self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    -self.penalty_area_width / 2.0
                ],
                point![
                    -self.length / 2.0 + self.penalty_area_length,
                    self.penalty_area_width / 2.0
                ],
            )),
            FieldLine::from_segment(LineSegment2::new(
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

/// Contains the coordinates for the starting positions for each robot.
/// This configuration assumes the center has coordinates (0, 0).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FieldPositionsConfig(Vec<RobotPosition>);

impl Index<usize> for FieldPositionsConfig {
    type Output = RobotPosition;

    // Required method
    fn index(&self, index: usize) -> &Self::Output {
        self.0
            .iter()
            .find(|elem| elem.player_number == index)
            .unwrap_or_else(|| panic!("Player index {index:?} not in layout configuration!"))
    }
}

impl FieldPositionsConfig {
    pub fn len(&self) -> usize {
        self.0.iter().len()
    }
}

impl FieldPositionsConfig {
    #[must_use]
    pub fn player(&self, player_num: u8) -> &RobotPosition {
        self.0
            .iter()
            .find(|elem| elem.player_number == player_num as usize)
            .unwrap_or_else(|| panic!("Player number {player_num:?} not in layout configuration!"))
    }
}

/// Contains the coordinates for one robot position.
/// Here it is assumed the centre point as coordinates (0, 0).
/// The x axis points towards the opponents' goal.
/// The y axis points towards the top (to the left with respect to  the x axis).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RobotPosition {
    /// Player number
    pub player_number: usize,

    // Position and orientation of the robot
    #[serde(deserialize_with = "isometry_with_angle::deserialize", flatten)]
    pub isometry: Isometry2<f32>,
}

impl Config for LayoutConfig {
    const PATH: &'static str = "layout.toml";
}
