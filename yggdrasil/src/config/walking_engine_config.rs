use odal::Configuration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct WalkingEngineConfig {}

impl Configuration for WalkingEngineConfig {
    const PATH: &'static str = "../config/walking_engine.toml";
}
