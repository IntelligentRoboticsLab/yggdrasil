use std::{
    any::type_name,
    fs::{self, read_to_string},
    path::Path,
};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::Table;

#[derive(Debug, Error, Diagnostic)]
#[error("Config `{name}` failed")]
pub struct Error {
    name: String,
    #[source]
    kind: ErrorKind,
}

impl Error {
    pub fn from_kind<T: Configuration>(kind: ErrorKind) -> Self {
        Self {
            name: T::name().to_string(),
            kind,
        }
    }
}

#[derive(Debug, Error, Diagnostic)]
pub enum ErrorKind {
    #[error("Failed to load from `{path}`")]
    ReadIo {
        path: String,
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
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Configuration: for<'de> Deserialize<'de> + Serialize {
    const PATH: &'static str;

    fn name() -> &'static str {
        type_name::<Self>()
    }

    /// Stores the configuration in a file at the specified path
    fn load(main_path: &Path, overlay_path: &Path) -> Result<Self> {
        let mut main: Table = read_to_string(main_path)
            .map_err(|e| {
                Error::from_kind::<Self>(ErrorKind::ReadIo {
                    path: main_path.display().to_string(),
                    source: e,
                })
            })?
            .parse()
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Deserialize(e)))?;

        let overlay: Option<Table> = match read_to_string(overlay_path) {
            Ok(toml_string) => Some(
                toml_string
                    .parse()
                    .map_err(|e| Error::from_kind::<Self>(ErrorKind::Deserialize(e)))?,
            ),
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => {
                tracing::debug!(
                    "`{}`: No overlay found at `{}`",
                    Self::name(),
                    overlay_path.display()
                );
                None
            }
            Err(e) => {
                return Err(Error::from_kind::<Self>(ErrorKind::ReadIo {
                    path: main_path.display().to_string(),
                    source: e,
                }));
            }
        };

        // merge overlay into main config if it exists
        if let Some(mut overlay) = overlay {
            Self::merge_tables(&mut main, &mut overlay);
        };

        Ok(main
            .try_into()
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Deserialize(e)))?)
    }

    /// Stores the configuration in a file at the specified path
    fn store(&self, path: &Path) -> Result<()> {
        let config_string = toml::to_string_pretty(self)
            .map_err(|e| Error::from_kind::<Self>(ErrorKind::Serialize(e.into())))?;

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
    /// # Note 🗒️:
    /// This function swaps values between tables and therefore leaves the overlay table in a garbage state.
    fn merge_tables(main: &mut Table, overlay: &mut Table) {
        for (key, value) in overlay.iter() {
            if !main.contains_key(key) {
                tracing::warn!(
                    "`{}`: Found key `{key}` = `{value}` in overlay which does not exist in main config", Self::name()
                );
            }
        }

        for (key, value) in main.iter_mut() {
            // overlay value must exist
            let Some(overlay_value) = overlay.get_mut(key) else {
                println!("not foundc");

                continue;
            };

            // they must be of the same type
            if std::mem::discriminant(value) != std::mem::discriminant(overlay_value) {
                tracing::warn!(
                    "`{}`: Type of value for key `{key}` is different between main config and overlay", Self::name()
                );

                continue;
            }

            // recursively merge tables
            if value.is_table() {
                Self::merge_tables(
                    value.as_table_mut().unwrap(),
                    overlay_value.as_table_mut().unwrap(),
                );
            } else {
                // place the overlay value in the main table
                std::mem::swap(value, overlay_value);
            }
        }
    }
}
