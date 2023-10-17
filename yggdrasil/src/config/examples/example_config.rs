use odal::{generate_config, Configuration};
use serde::{Deserialize, Serialize};
use std::fs;
use toml::Table;
use miette::Result;

/// Example configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PokemonConfig {
    bulbasaur: BulbasaurConfig,
    charmander: CharmanderConig,
}

impl Configuration for PokemonConfig {
    // Generates a robot specific config that can be added as a resource to the framework.
    // The generated config is created from the root config file and its overlay.
    fn load(path: &str) -> Self {
        // path to the root config
        let main_cfg: String = fs::read_to_string("../config/examples/example.toml").unwrap();
        let main_table: Table = toml::from_str(&main_cfg).unwrap();

        let overlay_cfg: String = fs::read_to_string(path).unwrap();
        let overlay_table: Table = toml::from_str(&overlay_cfg).unwrap();

        // This should be false when you just want to overwrite some existing values
        let add_key: bool = false;

        let generated_cfg: String = generate_config(main_table, overlay_table, add_key).to_string();
        let generated_toml: Self = toml::from_str(&generated_cfg).unwrap();

        generated_toml
    }

    /// Takes a path to an overlay and updates in a toml format (Table) and writes the requested
    /// changes to the overlay toml file.
    fn save(path: &str, updates: Table) {
        let overlay_cfg: String = fs::read_to_string(path).unwrap();
        let overlay_table: Table = toml::from_str(&overlay_cfg).unwrap();

        // This should be true when you could potentially add new keys to overlay
        let add_key: bool = true;

        let result = generate_config(overlay_table, updates, add_key);

        fs::write(path, result.to_string()).unwrap();
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BulbasaurConfig {
    hp: i32,
    attack: i32,
    defense: i32,
    speed: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CharmanderConfig {
    hp: i32,
    attack: i32,
    defense: i32,
    speed: i32,
    moves: CharmandersEpicMoves,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CharmandersEpicMoves {
    flamethrower: bool,
    growl: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        // Add the loaded config to the system
        .add_config::<PokemonConfig>("../examples/config/example.toml")
        .run();

    Ok(())
}