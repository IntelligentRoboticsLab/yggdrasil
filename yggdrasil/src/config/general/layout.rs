use odal::Config;
use serde::{Deserialize, Serialize};

/// Config that contains information about the layout of the field and
/// robot positions.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LayoutConfig {
    pub field: FieldConfig,
    pub initial_positions: InitialPositionsConfig,
}

/// Config that contains information about the field dimensions.
/// A schematic overview is given below:
///
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
///
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FieldConfig {
    /// Field length in millimeters (A)
    pub length: i32,
    /// Field width in millimeters (B)
    pub width: i32,
    /// Width of lines on the field (|)
    pub line_width: i32,
    /// Size of the penalty mark (0)
    pub penalty_mark_size: i32,
    /// Length of the goal area (E)
    pub goal_area_length: i32,
    /// Width of the goal area (F)
    pub goal_area_width: i32,
    /// Length of the penalty area (G)
    pub penalty_area_length: i32,
    /// Width of the penalty area (H)
    pub penalty_area_width: i32,
    /// Distance to the penalty mark from the side of the field (I)
    pub penalty_mark_distance: i32,
    /// Diameter of the centre circle (J)
    pub centre_circle_diameter: i32,
    /// Width of the border strip (K)
    pub border_strip_width: i32,
}

/// Containst the coordinates for the starting positions for each robot.
/// This configuration assumes the center has coordinates (0, 0).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InitialPositionsConfig {
    /// Position of the first robot
    one: RobotPosition,
    /// Position of the second robot
    two: RobotPosition,
    /// Position of the third robot
    three: RobotPosition,
    /// Position of the fourth robot
    four: RobotPosition,
    /// Position of the fifth robot
    five: RobotPosition,
}

/// Contains the coordinates for one robot position.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RobotPosition {
    /// Robot x-coordinate
    x: i32,
    /// Robot y-coordinate
    y: i32,
}

impl Config for LayoutConfig {
    const PATH: &'static str = "layout.toml";
}
