pub mod yggdrasil;

use std::path::PathBuf;

use crate::{nao::RobotInfo, prelude::*};

use odal::{Config, ConfigKind, Error, ErrorKind};

use miette::IntoDiagnostic;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        app.add_startup_system(initialize_config_roots)
    }
}

#[startup_system]
fn initialize_config_roots(storage: &mut Storage, info: &RobotInfo) -> miette::Result<()> {
    let main_path = PathBuf::from("./config/");
    let overlay_path = PathBuf::from(format!("./config/overlay/{}/", info.robot_name));

    let main = MainConfigRoot(main_path);
    let overlay = OverlayConfigRoot(overlay_path);

    storage.add_resource(Resource::new(main))?;
    storage.add_resource(Resource::new(overlay))?;

    Ok(())
}

#[derive(Debug)]
pub struct MainConfigRoot(PathBuf);

impl<T: Into<PathBuf>> From<T> for MainConfigRoot {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

#[derive(Debug)]
pub struct OverlayConfigRoot(PathBuf);

impl<T: Into<PathBuf>> From<T> for OverlayConfigRoot {
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
        self.add_startup_system(add_config::<T>)
    }
}

#[startup_system]
fn add_config<T: Config + Send + Sync + 'static>(
    storage: &mut Storage,
    main_path: &MainConfigRoot,
    overlay_path: &OverlayConfigRoot,
) -> miette::Result<()> {
    // add config file path to the config roots
    let main_path = main_path.0.join(T::PATH);
    let overlay_path = overlay_path.0.join(T::PATH);

    let config = match T::load_with_overlay(&main_path, &overlay_path) {
        Ok(t) => Ok(t),
        // failed to read overlay
        Err(Error {
            name,
            kind:
                ErrorKind::ReadIo {
                    path,
                    config_kind: ConfigKind::Overlay,
                    ..
                },
        }) => {
            tracing::debug!("`{name}`: Failed to read overlay from `{path}`");
            // use only root in that case
            T::load_without_overlay(&main_path).into_diagnostic()
        }
        Err(e) => Err(e.into()),
    }?;

    storage.add_resource(Resource::new(config))
}
