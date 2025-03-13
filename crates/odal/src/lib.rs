//! Odal helps you define configuration structs from toml files, overlay them, and catch silly🪿 mistakes while doing these things. 🗒️
mod error;

#[cfg(test)]
mod tests;

pub use error::*;

use std::{
    any::type_name,
    fs::{self, read_to_string},
    path::Path,
};

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

/// Trait that defines a configuration file for the implementor
pub trait Config: for<'de> Deserialize<'de> + Serialize {
    /// The relative path from which the configuration should be loaded
    const PATH: &'static str;

    /// The name of the configuration
    #[must_use]
    fn name() -> &'static str {
        type_name::<Self>()
    }

    /// Loads a configuration from a path
    ///
    /// # Errors
    ///
    /// This function will return an error if the configuration cannot be loaded.
    fn load(path: impl AsRef<Path>) -> Result<Self> {
        let main = load_table::<Self>(path.as_ref(), ConfigKind::Main)?;

        main.try_into()
            .map_err(|e| Error::deserialize::<Self>(path.as_ref(), &e))
    }

    /// Loads a configuration from two paths and overlays values from the second over the first
    ///
    /// # Errors
    ///
    /// This function will return an error if the configuration cannot be loaded or merged.
    fn load_with_overlay(
        main_path: impl AsRef<Path>,
        overlay_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let mut main = load_table::<Self>(main_path, ConfigKind::Main)?;
        let mut overlay = load_table::<Self>(overlay_path, ConfigKind::Overlay)?;

        merge_tables::<Self>(&mut main, &mut overlay)?;
        from_table::<Self>(main)
    }

    /// Stores the configuration in a file at the specified path
    ///
    /// # Errors
    ///
    /// This function will return an error if the configuration cannot be serialized or written to the file.
    fn store(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let config_string = toml::to_string_pretty(self)
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Serialize(e)))?;

        fs::write(path, config_string).map_err(|e| {
            Error::from_kind::<Self>(ErrorKind::Store {
                path: path.display().to_string(),
                source: e,
            })
        })?;

        Ok(())
    }
    
    /// Compares this config with the main config and saves the differences as an overlay
    ///
    /// # Errors
    ///
    /// This function will return an error if the overlay cannot be created or written to the file.
    fn save_as_overlay(&self, main: &Self, overlay_path: impl AsRef<Path>) -> Result<()> {
        save_as_overlay(main, self, overlay_path)
    }
}

/// Loads a configuration table from a path
fn load_table<T: Config>(path: impl AsRef<Path>, config_kind: ConfigKind) -> Result<Table> {
    let full_path = path.as_ref().join(T::PATH);

    let toml_string = read_to_string(&full_path).map_err(|e| {
        Error::from_kind::<T>(ErrorKind::Load {
            path: full_path.display().to_string(),
            config_kind,
            source: e,
        })
    })?;

    toml_string
        .parse()
        .map_err(|e| Error::deserialize::<T>(path.as_ref(), &e))
}

/// Overlay values from the overlay into the main table.
///
/// # Warning ⚠️
/// This function swaps values between tables and therefore leaves the overlay table in a garbage state.
fn merge_tables<T: Config>(main: &mut Table, overlay: &mut Table) -> Result<()> {
    // check if the overlay doesn't contain any keys that don't exist in the main overlay,
    // which might be indicative of an error made when configuring the overlay
    for (key, value) in overlay.iter() {
        if !main.contains_key(key) {
            return Err(Error::from_kind::<T>(ErrorKind::ExtraKey {
                key: key.to_string(),
                value: value.clone(),
            }));
        }
    }

    for (key, value) in main.iter_mut() {
        // try next key if there is no overlay value
        let Some(overlay_value) = overlay.get_mut(key) else {
            continue;
        };

        // values must be of the same type
        if std::mem::discriminant(value) != std::mem::discriminant(overlay_value) {
            return Err(Error::from_kind::<T>(ErrorKind::TypeMismatch {
                key: key.to_string(),
                main_value: value.clone(),
                overlay_value: overlay_value.clone(),
            }));
        }

        if value.is_table() {
            // recursively merge tables
            merge_tables::<T>(
                value.as_table_mut().unwrap(),
                overlay_value.as_table_mut().unwrap(),
            )
            .map_err(|e| {
                Error::from_kind::<T>(ErrorKind::Subtable {
                    key: key.clone(),
                    source: Box::new(e.kind),
                })
            })?;
        } else {
            // or replace main value with the overlay value
            std::mem::swap(value, overlay_value);
        }
    }

    Ok(())
}

/// Parses a [`Table`] into [`Self`]
fn from_table<T: Config>(table: Table) -> Result<T> {
    table
        .try_into()
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Parse(e)))
}

/// Extracts the differences between two TOML tables
///
/// Creates a new table containing only values that differ between the main and changed tables.
/// This is used to create an overlay config with only the changes.
fn extract_diff(main: &Table, changed: &Table) -> Table {
    let mut diff = Table::new();
    
    for (key, changed_value) in changed {
        if let Some(main_value) = main.get(key) {
            // If value is a nested table, recursively check for differences
            if main_value.is_table() && changed_value.is_table() {
                let main_subtable = main_value.as_table().unwrap();
                let changed_subtable = changed_value.as_table().unwrap();
                
                let subtable_diff = extract_diff(main_subtable, changed_subtable);
                
                // Only add the subtable if it has differences
                if !subtable_diff.is_empty() {
                    diff.insert(key.clone(), Value::Table(subtable_diff));
                }
            } 
            // If values are different, add to diff table
            else if main_value != changed_value {
                diff.insert(key.clone(), changed_value.clone());
            }
        }
        // Key exists in changed but not in main (should not happen with proper validation)
        else {
            diff.insert(key.clone(), changed_value.clone());
        }
    }
    
    diff
}

/// Extracts and saves changes between two configurations as an overlay
///
/// Compares the `changed` configuration against the `main` configuration
/// and creates an overlay file containing only the differences.
///
/// # Examples
///
/// ```no_run
/// use serde::{Deserialize, Serialize};
/// use odal::Config;
/// use std::path::Path;
///
/// #[derive(Deserialize, Serialize, Clone)]
/// struct TestConfig {
///     value: i32,
///     nested: NestedConfig,
/// }
///
/// #[derive(Deserialize, Serialize, Clone)]
/// struct NestedConfig {
///     setting: String,
/// }
///
/// impl Config for TestConfig {
///     const PATH: &'static str = "test.toml";
/// }
///
/// // Load main config
/// let main = TestConfig::load(Path::new("./config")).unwrap();
///
/// // Create a modified version
/// let mut modified = main.clone();
/// modified.nested.setting = "new value".to_string();
/// 
/// // Save only the changes as an overlay
/// modified.save_as_overlay(&main, Path::new("./config/overlay/robot")).unwrap();
/// ```
///
/// # Errors
///
/// This function will return an error if the overlay cannot be created or written to the file.
pub fn save_as_overlay<T: Config>(
    main: &T,
    changed: &T,
    overlay_path: impl AsRef<Path>,
) -> Result<()> {
    let main_value = toml::to_string_pretty(main)
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Serialize(e)))?;
    
    let changed_value = toml::to_string_pretty(changed)
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Serialize(e)))?;

    let main_table: Table = main_value
        .parse()
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Parse(e)))?;
    
    let changed_table: Table = changed_value
        .parse()
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Parse(e)))?;

    let overlay_table = extract_diff(&main_table, &changed_table);
    
    // Create directory if it doesn't exist
    if let Some(parent) = overlay_path.as_ref().parent() {
        fs::create_dir_all(parent).map_err(|e| {
            Error::from_kind::<T>(ErrorKind::Store {
                path: parent.display().to_string(),
                source: e,
            })
        })?;
    }

    // Write the overlay file
    let overlay_string = toml::to_string_pretty(&overlay_table)
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Serialize(e)))?;
    
    let path = overlay_path.as_ref().join(T::PATH);

    fs::write(&path, overlay_string).map_err(|e| {
        Error::from_kind::<T>(ErrorKind::Store {
            path: path.display().to_string(),
            source: e,
        })
    })?;

    Ok(())
}
