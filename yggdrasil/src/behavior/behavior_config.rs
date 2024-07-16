use odal::Config;
use serde::{Deserialize, Serialize};
use tyr::Inspect;

use super::behaviors::ObserveBehaviorConfig;

/// Config that contains information about the layout of the field and
/// robot positions.
#[derive(Serialize, Deserialize, Inspect, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BehaviorConfig {
    pub observe: ObserveBehaviorConfig,
}

impl Config for BehaviorConfig {
    const PATH: &'static str = "behavior.toml";
}
