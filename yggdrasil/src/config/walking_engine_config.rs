use serde::{Deserialize, Serialize};
use toml::Table;
use std::fs;
use odal::{Configuration, generate_config};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct WalkingEngineConfig {
    #[serde(default)]
    pub balance: BalanceConfig,
    #[serde(default)]
    pub speed: SpeedConfig,
}

impl Configuration for WalkingEngineConfig {
    fn load(path: &str) -> Self {

        let main_cfg: String = fs::read_to_string("../config/walking_engine.toml").unwrap();
        let main_table: Table = toml::from_str(&main_cfg).unwrap();

        let overlay_cfg: String = fs::read_to_string(path).unwrap();
        let overlay_table: Table = toml::from_str(&overlay_cfg).unwrap();

        let add_key: bool = false;

        let generated_cfg: String = generate_config(main_table, overlay_table, add_key).to_string();
        let generated_toml: Self = toml::from_str(&generated_cfg).unwrap();

        generated_toml
    }

    fn store(path: &str, updates: Table) {
        let overlay_cfg: String = fs::read_to_string(path).unwrap();
        let overlay_table: Table = toml::from_str(&overlay_cfg).unwrap();

        let add_key: bool = true;

        let result = generate_config(overlay_table, updates, add_key);

        fs::write(path, result.to_string()).unwrap();
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BalanceConfig;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SpeedConfig;