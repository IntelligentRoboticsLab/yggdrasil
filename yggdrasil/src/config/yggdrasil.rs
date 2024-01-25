use odal::Configuration;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct YggdrasilConfig {
    pub number: i32,
    name: String,
    table: Inner,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Inner {
    pub number: i32,
}

impl Configuration for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";

    fn name() -> &'static str {
        "YggdrasilConfig"
    }
}
