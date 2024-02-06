use std::{fmt::Display, fs::read_to_string, path::Path};

use miette::NamedSource;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use toml::Value;

use crate::Config;

/// The kind of config: main or overlay
///
/// The main config is a global configuration that applies to all robots. Optionally, an overlay config can be provided for a specific robot, which includes overrides for the global default.
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
    #[error("Failed to load {config_kind} config from `{path}`")]
    Load {
        path: String,
        config_kind: ConfigKind,
        source: std::io::Error,
    },
    #[error("Failed to store at `{path}`")]
    Store {
        path: String,
        source: std::io::Error,
    },
    #[error("Failed to seralize toml")]
    Serialize(#[from] toml::ser::Error),
    #[error("Failed to deserialize toml:\n{message}\n")]
    Deserialize {
        #[source_code]
        definition_source: NamedSource,
        #[label("Failed here")]
        parse_error_pos: Option<SourceSpan>,
        message: String,
    },
    #[error("Failed to parse table into struct")]
    Parse(#[from] toml::de::Error),
    #[error("Found key `{key}` in overlay that does not exist in main config")]
    ExtraKey { key: String, value: Value },
    #[error("Type of value is different between main config and overlay for key `{key}`")]
    TypeMismatch {
        key: String,
        main_value: Value,
        overlay_value: Value,
    },
    #[error("Failed to parse subtable `{key}` in overlay")]
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

    pub fn deserialize<T: Config>(path: impl AsRef<Path>, source: toml::de::Error) -> Self {
        let path = path.as_ref().join(T::PATH);
        let toml_string = read_to_string(&path).unwrap();

        Self::from_kind::<T>(ErrorKind::Deserialize {
            definition_source: NamedSource::new(path.to_string_lossy(), toml_string),
            parse_error_pos: source.span().map(Into::into),
            message: source.message().to_string(),
        })
    }
}

/// Result type that returns an [`struct@Error`]
pub type Result<T> = std::result::Result<T, Error>;
