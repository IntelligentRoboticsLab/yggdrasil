use miette::Result;
use odal::{ConfigResource, Configuration};
use serde::{Deserialize, Serialize};
use tyr::prelude::*;

/// Example configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
struct PokemonConfig {
    bulbasaur: BulbasaurConfig,
    charmander: CharmanderConfig,
}

impl Configuration for PokemonConfig {
    // Specify the path to a root config.
    const PATH: &'static str = "../config/examples/example.toml";
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

// Retrieve and use a config value from storage.
impl CharmanderConfig {
    fn add_hp(&mut self, hp_amount: i32) {
        self.hp += hp_amount;
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        // Adds a loaded config to the system at runtime.
        .add_config::<PokemonConfig>("../examples/config/overlays/charmander/example.toml")?
        .run()?;

    Ok(())
}
