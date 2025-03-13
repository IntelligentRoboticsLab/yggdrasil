use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};

use super::behaviors::{
    ObserveBehaviorConfig, RlDefenderDribbleBehaviorConfig, RlDefenderSearchBehaviorConfig,
    RlStrikerSearchBehaviorConfig,
};

/// Config that contains information about the layout of the field and
/// robot positions.
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BehaviorConfig {
    pub observe: ObserveBehaviorConfig,
    pub rl_striker_search: RlStrikerSearchBehaviorConfig,
    pub rl_defender_search: RlDefenderSearchBehaviorConfig,
    pub rl_defender_dribble: RlDefenderDribbleBehaviorConfig,
}

impl Config for BehaviorConfig {
    const PATH: &'static str = "behavior.toml";
}
