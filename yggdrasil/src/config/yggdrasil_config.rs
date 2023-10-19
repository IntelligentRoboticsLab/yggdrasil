use odal::Configuration;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct YggdrasilConfig {}

impl Configuration for YggdrasilConfig {
    const PATH: &'static str = "../config/yggdrasil.toml";
}
