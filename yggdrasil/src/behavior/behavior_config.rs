use odal::Config;
use serde::{Deserialize, Serialize};

use crate::behavior::behaviors::InitialBehaviorConfig;

/// Config that contains information about the layout of the field and
/// robot positions.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BehaviorConfig {
    pub initial_behaviour: InitialBehaviorConfig,
}

impl Config for BehaviorConfig {
    const PATH: &'static str = "behavior.toml";
}
