use toml::Table;

pub trait Configuration {
    fn load(path: &str) -> Self;
    fn store(path: &str, parameters: Table);
}

pub fn generate_config(main: Table, overlay: Table, add_key: bool) -> Table {
    let mut generated_toml: Table = Table::new();

    // Process keys in main table
    for (k, v) in main.clone() {
        match overlay.get(&k) {
            Some(overlay_value) => {
                // If the key exists in overlay, check if it's also a table
                if let toml::Value::Table(main_table) = v {
                    if let toml::Value::Table(overlay_table) = overlay_value {
                        // Recursively merge the subtables
                        let merged_table = generate_config(main_table.clone(), overlay_table.clone(), add_key);
                        generated_toml.insert(k.clone(), toml::Value::Table(merged_table));
                    }
                } else {
                    // If it's not a table, use the overlay value
                    generated_toml.insert(k.clone(), overlay_value.clone());
                }
            }
            None => {
                // If the key doesn't exist in overlay, use the main value
                generated_toml.insert(k.clone(), v.clone());
            }
        }
    }

    // Process keys in overlay that don't exist in main
    if add_key {
        for (k, v) in overlay {
            if !main.contains_key(&k) {
                generated_toml.insert(k.clone(), v.clone());
            }
        }
    }

    generated_toml
}