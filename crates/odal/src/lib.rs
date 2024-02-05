//! Odal helps you define configuration structs from toml files, overlay them, and catch silly🪿 mistakes while doing these things. 🗒️
mod error;

pub use error::*;

use std::{
    any::type_name,
    fs::{self, read_to_string},
    path::Path,
};

use serde::{Deserialize, Serialize};
use toml::Table;

/// Trait that defines a configuration file for the implementor
pub trait Config: for<'de> Deserialize<'de> + Serialize {
    /// The relative path from which the configuration should be loaded
    const PATH: &'static str;

    /// The name of the configuration
    fn name() -> &'static str {
        type_name::<Self>()
    }

    /// Loads a configuration from a path
    fn load(path: impl AsRef<Path>) -> Result<Self> {
        let main = load_table::<Self>(path.as_ref(), ConfigKind::Main)?;

        main.try_into()
            .map_err(|e| Error::deserialize::<Self>(path.as_ref(), e))
    }

    /// Loads a configuration from two paths and overlays values from the second over the first
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
        .map_err(|e| Error::deserialize::<T>(path.as_ref(), e))
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
