//! Odal helps you define configuration structs from toml files, overlay them, and catch silly🪿 mistakes while doing these things. 🗒️

use std::{
    any::type_name,
    fmt::Display,
    fs::{self, read_to_string},
    path::Path,
};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::{Table, Value};

/// The kind of config: main or overlay
#[derive(Debug)]
pub enum ConfigKind {
    Main,
    Overlay,
}

impl Display for ConfigKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ConfigKind::Main => "main",
            ConfigKind::Overlay => "overlay",
        };

        f.write_str(name)
    }
}

/// Error kinds that can occur when using odal configs
#[derive(Debug, Error, Diagnostic)]
pub enum ErrorKind {
    #[error("Found key `{key}` in overlay that does not exist in main config")]
    ExtraKey { key: String, value: Value },
    #[error("Type of value is different between main config and overlay for key `{key}`")]
    TypeMismatch {
        key: String,
        main_value: Value,
        overlay_value: Value,
    },
    #[error("Failed to load {config_kind} config from `{path}`")]
    ReadIo {
        path: String,
        config_kind: ConfigKind,
        source: std::io::Error,
    },
    #[error("Failed to store at `{path}`")]
    StoreIo {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to seralize toml")]
    Serialize(#[from] toml::ser::Error),
    #[error("Failed to deseralize toml")]
    Deserialize(#[from] toml::de::Error),
    #[error("Invalid subtable `{key}` in overlay")]
    Subtable { key: String, source: Box<ErrorKind> },
}

/// Error type for an odal config
#[derive(Debug, Error, Diagnostic)]
#[error("Config `{name}` failed")]
pub struct Error {
    pub name: String,
    #[source]
    pub kind: ErrorKind,
}

impl Error {
    /// Create an error that automatically inserts the config name
    pub fn from_kind<T: Config>(kind: ErrorKind) -> Self {
        Self {
            name: T::name().to_string(),
            kind,
        }
    }
}

/// Result type that returns an [`struct@Error`]
pub type Result<T> = std::result::Result<T, Error>;

/// Trait that defines a configuration file for the implementor
pub trait Config: for<'de> Deserialize<'de> + Serialize {
    /// The relative path from which the configuration should be loaded
    const PATH: &'static str;

    /// The name of the configuration
    fn name() -> &'static str {
        type_name::<Self>()
    }

    /// Loads a configuration from a path
    fn load_without_overlay(path: impl AsRef<Path>) -> Result<Self> {
        let main = load_table::<Self>(path, ConfigKind::Main)?;

        main.try_into()
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Deserialize(e)))
    }

    /// Loads a configuration from two paths and overlays values from the second over the first
    fn load_with_overlay(
        main_path: impl AsRef<Path>,
        overlay_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let mut main = load_table::<Self>(main_path, ConfigKind::Main)?;
        let mut overlay = load_table::<Self>(overlay_path, ConfigKind::Overlay)?;

        Self::merge_tables(&mut main, &mut overlay)?;

        main.try_into()
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Deserialize(e)))
    }

    /// Stores the configuration in a file at the specified path
    fn store(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        let config_string = toml::to_string_pretty(self)
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Serialize(e)))?;

        fs::write(path, config_string).map_err(|e| {
            Error::from_kind::<Self>(ErrorKind::StoreIo {
                path: path.display().to_string(),
                source: e,
            })
        })?;

        Ok(())
    }

    /// Overlay values from the overlay into the main table.
    ///
    /// # Warning ⚠️
    /// This function swaps values between tables and therefore leaves the overlay table in a garbage state.
    fn merge_tables(main: &mut Table, overlay: &mut Table) -> Result<()> {
        // check if the overlay doesn't contain any keys that don't exist in the main overlay,
        // which might be indicative of an error made when configuring the overlay
        for (key, value) in overlay.iter() {
            if !main.contains_key(key) {
                return Err(Error::from_kind::<Self>(ErrorKind::ExtraKey {
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
                return Err(Error::from_kind::<Self>(ErrorKind::TypeMismatch {
                    key: key.to_string(),
                    main_value: value.clone(),
                    overlay_value: overlay_value.clone(),
                }));
            }

            if value.is_table() {
                // recursively merge tables
                Self::merge_tables(
                    value.as_table_mut().unwrap(),
                    overlay_value.as_table_mut().unwrap(),
                )
                .map_err(|e| {
                    Error::from_kind::<Self>(ErrorKind::Subtable {
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
}

/// Loads a configuration table from a path
fn load_table<T: Config>(path: impl AsRef<Path>, config_kind: ConfigKind) -> Result<Table> {
    let full_path = path.as_ref().join(T::PATH);

    read_to_string(&full_path)
        .map_err(|e| {
            Error::from_kind::<T>(ErrorKind::ReadIo {
                path: full_path.display().to_string(),
                config_kind,
                source: e,
            })
        })?
        .parse()
        .map_err(|e| Error::from_kind::<T>(ErrorKind::Deserialize(e)))
}
