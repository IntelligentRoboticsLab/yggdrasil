pub mod tyr;

use std::path::{Path, PathBuf};

use crate::{nao::RobotInfo, prelude::*};

use ::tyr::tasks::TaskModule;
use odal::{ConfigKind, Error, ErrorKind};

use tyr::TyrConfig;

/// This module adds functionality to load configuration structs from files.
///
/// It provides the following resources to the application:
/// - [`MainConfigDir`]
/// - [`OverlayConfigDir`]
///
/// # Example
///
/// ```no_run
/// use yggdrasil::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Deserialize, Serialize)]
/// #[serde(deny_unknown_fields)]
/// pub struct MeowConfig {
///     count: u32,
/// }
///
/// pub struct MeowModule;
///
/// impl Config for MeowConfig {
///     const PATH: &'static str = "meow.toml";
/// }
///
/// impl Module for MeowModule {
///     fn initialize(self, app: App) -> Result<App> {
///         app.init_config::<MeowConfig>()
///     }
/// }
/// ```
///
pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        app.add_startup_system(initialize_config_roots)?
            .init_config::<TyrConfig>()?
            .add_startup_system(tyr::configure_tyr_hack)?
            .add_module(TaskModule)
    }
}

#[startup_system]
fn initialize_config_roots(storage: &mut Storage, info: &RobotInfo) -> miette::Result<()> {
    let main_dir = PathBuf::from("./config/");
    let overlay_dir = PathBuf::from(format!("./config/overlay/{}/", info.robot_name));

    assert!(main_dir.is_dir());
    assert!(overlay_dir.is_dir());

    let main = MainConfigDir(main_dir);
    let overlay = OverlayConfigDir(overlay_dir);

    storage.add_resource(Resource::new(main))?;
    storage.add_resource(Resource::new(overlay))?;

    Ok(())
}

#[derive(Debug)]
/// Directory where the main configs are stored
pub struct MainConfigDir(PathBuf);

impl<T: Into<PathBuf>> From<T> for MainConfigDir {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[derive(Debug)]
/// Directory where the overlay configs are stored
pub struct OverlayConfigDir(PathBuf);

impl<T: Into<PathBuf>> From<T> for OverlayConfigDir {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Trait for adding configs to an [`App`]
pub trait ConfigResource {
    /// Adds the configuration `T` to the app
    fn init_config<T: Config + Send + Sync + 'static>(self) -> miette::Result<Self>
    where
        Self: Sized;
}

impl ConfigResource for App {
    fn init_config<T: Config + Send + Sync + 'static>(self) -> miette::Result<Self>
    where
        Self: Sized,
    {
        self.add_startup_system(_init_config::<T>)
    }
}

#[startup_system]
fn _init_config<T: Config + Send + Sync + 'static>(
    storage: &mut Storage,
    main_dir: &MainConfigDir,
    overlay_dir: &OverlayConfigDir,
) -> crate::Result<()> {
    // add config file path to the config roots
    let main_path: &Path = main_dir.0.as_ref();
    let overlay_path: &Path = overlay_dir.0.as_ref();

    let config = match T::load_with_overlay(main_path, overlay_path) {
        Ok(t) => Ok(t),
        // failed to load any overlay
        Err(Error {
            name,
            kind:
                ErrorKind::Load {
                    path,
                    config_kind: ConfigKind::Overlay,
                    ..
                },
        }) => {
            // log and use only main config
            tracing::debug!("`{name}`: Failed to read overlay from `{path}`");
            // use only root in that case
            T::load(main_path)
        }
        Err(e) => Err(e),
    }?;

    storage.add_resource(Resource::new(config))
}
