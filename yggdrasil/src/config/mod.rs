pub mod yggdrasil;

use std::path::PathBuf;

use odal::{Config, ConfigKind, Error, ErrorKind};
use tyr::prelude::*;

use crate::nao::RobotInfo;

use miette::IntoDiagnostic;
use yggdrasil::TyrModule;

pub struct ConfigModule;

impl Module for ConfigModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app
            .add_startup_system(initialize_config_roots)?
            .add_system(|a: Res<TyrModule>| {
                println!("{:#?}", *a);
                std::thread::sleep(std::time::Duration::from_secs(1));
                Ok(())
            }))
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
    fn init_module<T: Module + Config>(self) -> miette::Result<Self>
    where
        Self: Sized;

    /// Adds the configuration `T` to the app
    fn init_config<T: Config + Send + Sync + 'static>(self) -> miette::Result<Self>
    where
        Self: Sized;
}

impl ConfigResource for App {
    fn init_module<T: Module + Config>(self) -> miette::Result<Self>
    where
        Self: Sized,
    {
        {
            let module = load_config::<T>(&self)?;

            self.add_module(module)
        }
    }

    fn init_config<T: Config + Send + Sync + 'static>(self) -> miette::Result<Self>
    where
        Self: Sized,
    {
        let config = load_config::<T>(&self)?;

        self.add_resource(Resource::new(config))
    }
}

#[startup_system]
fn add_config<T: Configuration + Send + Sync + 'static>(
    storage: &mut Storage,
    main_path: &MainConfigRoot,
    overlay_path: &OverlayConfigRoot,
) -> miette::Result<()> {
    // add config file path to the config roots
    let main_path = main_path.0.join(T::PATH);
    let overlay_path = overlay_path.0.join(T::PATH);

    let config = YggdrasilConfig::load(&main_path, &overlay_path)?;

    storage.add_resource(Resource::new(config))?;

    Ok(())
}
