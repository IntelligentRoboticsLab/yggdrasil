use std::ops::Index;

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

/// Contains the coordinates for the starting positions for each robot.
/// This configuration assumes the center has coordinates (0, 0).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct InitialPositionsConfig(Vec<RobotPosition>);

impl Index<usize> for InitialPositionsConfig {
    type Output = RobotPosition;

    // Required method
    fn index(&self, index: usize) -> &Self::Output {
        self.0
            .iter()
            .find(|elem| elem.player_number == index)
            .expect("Player number not in layout configuration!")
    }
}

impl InitialPositionsConfig {
    pub fn player(&self, player_num: u8) -> &RobotPosition {
        self.0
            .iter()
            .find(|elem| elem.player_number == player_num as usize)
            .expect("Player number not in layout configuration!")
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
    /// Robot x-coordinate in metres.
    pub x: f32,
    /// Robot y-coordinate in metres.
    pub y: f32,

    pub rotation: f32,
}

impl Config for LayoutConfig {
    const PATH: &'static str = "layout.toml";
}
