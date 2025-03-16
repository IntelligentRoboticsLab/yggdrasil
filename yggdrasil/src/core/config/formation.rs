use bevy::prelude::*;
use nalgebra::Isometry2;
use odal::Config;
use serde::{Deserialize, Serialize};
use std::ops::Index;

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

/// Config that contains information about the robot positions.
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FormationConfig {
    pub initial_positions: FieldPositionsConfig,
    pub set_positions: FieldPositionsConfig,

    #[serde(deserialize_with = "isometry_with_angle::deserialize_vec")]
    pub penalty_positions: Vec<Isometry2<f32>>,
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
            .unwrap_or_else(|| panic!("Player index {index:?} not in formation configuration!"))
    }
}

impl FieldPositionsConfig {
    #[must_use]
    pub fn player(&self, player_num: u8) -> &RobotPosition {
        self.0
            .iter()
            .find(|elem| elem.player_number == player_num as usize)
            .unwrap_or_else(|| {
                panic!("Player number {player_num:?} not in formation configuration!")
            })
    }
}

/// Contains the coordinates for one robot position.
/// Here it is assumed the centre point as coordinates (0, 0).
/// The x axis points towards the opponents' goal.
/// The y axis points towards the top (to the left with respect to the x axis).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RobotPosition {
    /// Player number
    pub player_number: usize,

    // Position and orientation of the robot
    #[serde(deserialize_with = "isometry_with_angle::deserialize", flatten)]
    pub isometry: Isometry2<f32>,
}

impl Config for FormationConfig {
    const PATH: &'static str = "formation.toml";
}
