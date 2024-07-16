use std::ops::Index;

use nalgebra::Isometry2;
use nalgebra::Vector2;
use odal::Config;
use serde::{Deserialize, Serialize};

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
#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl FieldConfig {
    pub fn diagonal(&self) -> Vector2<f32> {
        Vector2::new(self.length, self.width)
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
            .unwrap_or_else(|| panic!("Player index {:?} not in layout configuration!", index))
    }
}

impl FieldPositionsConfig {
    pub fn player(&self, player_num: u8) -> &RobotPosition {
        self.0
            .iter()
            .find(|elem| elem.player_number == player_num as usize)
            .unwrap_or_else(|| {
                panic!(
                    "Player number {:?} not in layout configuration!",
                    player_num
                )
            })
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
