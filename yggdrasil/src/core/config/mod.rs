pub mod layout;
pub mod showtime;
pub mod tyr;
pub mod yggdrasil;

use std::path::{Path, PathBuf};

use crate::{behavior::BehaviorConfig, nao::RobotInfo, prelude::*};
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use odal::{ConfigKind, Error, ErrorKind};

use layout::LayoutConfig;
use showtime::ShowtimeConfig;
use tyr::TyrConfig;
use yggdrasil::YggdrasilConfig;

/// Plugin that adds functionality to load configuration structs from files.
///
/// It provides the following resources to the application:
/// - [`MainConfigDir`]
/// - [`OverlayConfigDir`]
///
/// # Example
///
/// ```no_run
/// use bevy::prelude::*;
/// use yggdrasil::prelude::*;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Resource, Debug, Deserialize, Serialize)]
/// #[serde(deny_unknown_fields)]
/// pub struct MeowConfig {
///     count: u32,
/// }
///
/// pub struct MeowPlugin;
///
/// impl Config for MeowConfig {
///     const PATH: &'static str = "meow.toml";
/// }
///
/// impl Plugin for MeowPlugin {
///     fn build(&self, app: &mut App){
///         // This will load the configuration from `config/meow.toml`
///         // and insert it into the world as a resource.
///         app.init_config::<MeowConfig>();
///     }
/// }
/// ```
///
pub struct ConfigPlugin;

impl Plugin for ConfigPlugin {
    fn build(&self, app: &mut App) {
        let robot_info = app.world().resource::<RobotInfo>();
        let main_dir = PathBuf::from("./config/");
        let overlay_dir = PathBuf::from(format!("./config/overlay/{}/", robot_info.robot_name));

        assert!(main_dir.is_dir(), "main config directory does not exist");
        assert!(
            overlay_dir.is_dir(),
            "overlay config directory for {} does not exist",
            robot_info.robot_name
        );

        app.insert_resource(MainConfigDir(main_dir))
            .insert_resource(OverlayConfigDir(overlay_dir));

        app.init_config::<ShowtimeConfig>()
            .init_config::<LayoutConfig>()
            .init_config::<BehaviorConfig>()
            .init_config::<TyrConfig>()
            .init_config::<YggdrasilConfig>();

        app.add_systems(
            PreStartup,
            (init_subconfigs, showtime::configure_showtime).chain(),
        );
    }
}

fn init_subconfigs(mut commands: Commands, config: Res<YggdrasilConfig>) {
    // commands.insert_resource(config.camera.clone());
    commands.insert_resource(config.filter.clone());
    commands.insert_resource(config.game_controller.clone());
    commands.insert_resource(config.primary_state.clone());
    // commands.insert_resource(config.vision.field_marks.clone());
    commands.insert_resource(config.odometry.clone());
    commands.insert_resource(config.orientation.clone());
}

/// Directory where the main configs are stored
#[derive(Resource, Debug)]
pub struct MainConfigDir(PathBuf);

impl<T: Into<PathBuf>> From<T> for MainConfigDir {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Directory where the overlay configs are stored
#[derive(Resource, Debug)]
pub struct OverlayConfigDir(PathBuf);

impl<T: Into<PathBuf>> From<T> for OverlayConfigDir {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// Trait for adding configs to an [`App`]
pub trait ConfigExt {
    /// Adds the configuration `T` to the app
    fn init_config<T: Resource + Config + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        Self: Sized;
}

impl ConfigExt for App {
    fn init_config<T: Resource + Config + Send + Sync + 'static>(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self.world_mut().run_system_once(init_config::<T>);
        self
    }
}

fn init_config<T: Resource + Config + Send + Sync + 'static>(
    mut commands: Commands,
    main_dir: Res<MainConfigDir>,
    overlay_dir: Res<OverlayConfigDir>,
) {
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
    }
    .expect(&format!("failed to load config: {}", T::PATH));

    commands.insert_resource(config);
}
