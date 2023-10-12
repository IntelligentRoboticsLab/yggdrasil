use odal::{generate_config, Configuration};
use serde::{Deserialize, Serialize};
use std::fs;
use toml::Table;

/// Example configuration
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PokemonConfig {
    #[serde(default)]
    pub bulbasaur: BulbasaurConfig,
    #[serde(default)]
    pub charmander: CharmanderConig,
    #[serde(default)]
    pub squirtle: SquirtleConfig,
}

impl Configuration for PokemonConfig {
    fn load(path: &str) -> Self {
        let main_cfg: String = fs::read_to_string("../config/yggdrasil.toml").unwrap();
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
pub struct BulbasaurConfig {
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub speed: i32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct CharmanderConfig {
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub speed: i32,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct SquirtleConfig {
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub speed: i32,
}
