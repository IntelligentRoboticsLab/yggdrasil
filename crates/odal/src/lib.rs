use miette::Result;
use std::fs;
use toml::Table;
use toml::{from_str, macros::Deserialize, Value};
use tyr::prelude::*;

/// Configuration provides functionality for saving and loading configurations.
pub trait Configuration: Sized + for<'de> Deserialize<'de> {
    /// Path to a root config
    const PATH: &'static str;

    /// Generates a config from an overlay and a root config.
    fn load(overlay_path: &str) -> Self {
        let main_cfg: Table = from_str(&fs::read_to_string(Self::PATH).unwrap()).unwrap();
        let overlay_cfg: Table = from_str(&fs::read_to_string(overlay_path).unwrap()).unwrap();
        let generated_cfg: Self =
            from_str(&generate_config(main_cfg, overlay_cfg, false).to_string()).unwrap();
        generated_cfg
    }

    /// Adds new K/V pairs to an overlay or overwrites existing pairs with the requested updates.
    fn save(overlay_path: &str, updates: Table) {
        let overlay_cfg: Table = from_str(&fs::read_to_string(overlay_path).unwrap()).unwrap();
        let result = generate_config(overlay_cfg, updates, true);
        fs::write(overlay_path, result.to_string()).unwrap();
    }
}

/// ConfigResource provides functionality for adding configurations as resources to App.
pub trait ConfigResource {
    /// Wrapper around Resource in order to add configurations to the app.
    fn add_config<T: Configuration + Send + Sync + 'static + for<'de> Deserialize<'de>>(
        self,
        path: &str,
    ) -> Result<Self>
    where
        Self: Sized;
}

impl ConfigResource for App {
    fn add_config<T: Configuration + Send + Sync + 'static + for<'de> Deserialize<'de>>(
        self,
        path: &str,
    ) -> Result<Self>
    where
        Self: Sized,
    {
        self.add_resource(Resource::new(T::load(path)))
    }
}

pub fn generate_config(main: Table, overlay: Table, add_keys: bool) -> Table {
    let mut generated_toml: Table = Table::new();

    // Process keys in main table
    for (k, v) in main.clone() {
        match overlay.get(&k) {
            Some(overlay_value) => {
                // If the key exists in overlay, check if it's also a table
                if let Value::Table(main_table) = v {
                    if let Value::Table(overlay_table) = overlay_value {
                        // Recursively merge the subtables
                        let merged_table =
                            generate_config(main_table, overlay_table.clone(), add_keys);
                        generated_toml.insert(k, Value::Table(merged_table));
                    }
                } else {
                    // If it's not a table, use the overlay value
                    generated_toml.insert(k, overlay_value.clone());
                }
            }
            None => {
                // If the key doesn't exist in overlay, use the main value
                generated_toml.insert(k, v);
            }
        }
    }

    // Process keys in overlay that don't exist in main
    if add_keys {
        for (k, v) in overlay {
            if !main.contains_key(&k) {
                generated_toml.insert(k, v);
            }
        }
    }

    generated_toml
}
