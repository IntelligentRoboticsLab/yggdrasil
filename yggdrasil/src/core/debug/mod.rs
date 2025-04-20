pub mod debug_system;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use miette::IntoDiagnostic;
use re_control_comms::debug_system::DebugEnabledSystems;
use rerun::{
    Angle, AsComponents, DEFAULT_SERVER_PORT, EntityPath, RecordingStream,
    SerializedComponentColumn, TimeColumn,
};
use std::convert::Into;
use std::env;
use std::f32::consts::PI;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{marker::PhantomData, net::IpAddr};

use crate::{
    nao::{Cycle, CycleTime},
    prelude::Result,
};

const DEFAULT_STORAGE_PATH: &str = "/mnt/usb";
const STORAGE_PATH_ENV_NAME: &str = "RERUN_STORAGE_PATH";
const DATE_TIME_FORMAT: &str = "%Y_%m_%d-%H_%M_%S";

/// Plugin that adds debugging tools for the robot using the [rerun](https://rerun.io) viewer.
///
/// This introduces a [`DebugContext`] [`SystemParam`], which can be used
/// for common debugging tasks.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugEnabledSystems>()
            .add_systems(Startup, (init_rerun, setup_spl_field).chain())
            .add_systems(First, sync_cycle_number);
    }
}

fn get_storage_path() -> Option<PathBuf> {
    env::var_os(STORAGE_PATH_ENV_NAME).map_or_else(
        || {
            let usb_path = PathBuf::from(DEFAULT_STORAGE_PATH);
            if usb_path.exists() {
                Some(usb_path)
            } else {
                None
            }
        },
        |path| Some(PathBuf::from(path)),
    )
}

fn make_rrd_file_path(storage_path: &Path) -> PathBuf {
    if !storage_path.is_dir() {
        return storage_path.into();
    }
    let mut path = PathBuf::new();

    path.push(
        storage_path
            .to_str()
            .expect("rerun rrd file path contains invalid unicode"),
    );
    path.push(chrono::Local::now().format(DATE_TIME_FORMAT).to_string());
    path.set_extension("rrd");

    path
}

pub fn init_rerun(mut commands: Commands) {
    #[cfg(feature = "local")]
    let server_address = Some(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));

    // Manually set the server address to the robot's IP address, instead of 0.0.0.0
    // to ensure the rerun server prints the correct connection URL on startup
    #[cfg(not(feature = "local"))]
    let server_address = {
        let host = std::env::var("RERUN_HOST").into_diagnostic();

        host.ok()
            .and_then(|address| std::str::FromStr::from_str(address.as_str()).ok())
    };

    let rec = if let Some(storage_path) = get_storage_path() {
        let output_rrd_file_path = make_rrd_file_path(&storage_path);
        tracing::info!(
            "Rerun log sink set to file: {}",
            output_rrd_file_path.as_path().display()
        );
        RerunStream::init_file_sink("yggdrasil", output_rrd_file_path)
            .expect("failed to initialize rerun::RecordingStream")
    } else if let Some(address) = server_address {
        RerunStream::init_grpc_server("yggdrasil", address)
            .expect("failed to initialize rerun::RecordingStream")
    } else {
        tracing::warn!("`RERUN_HOST` not set, rerun debugging is disabled");
        RerunStream::disabled()
    };

    commands.insert_resource(rec);
}

fn setup_spl_field(dbg: DebugContext) {
    dbg.log_static(
        "field",
        &rerun::Asset3D::from_file_path("./assets/rerun/field.glb")
            .expect("Failed to load field model")
            .with_media_type(rerun::MediaType::glb()),
    );

    dbg.log_static(
        "field/goals",
        &rerun::Asset3D::from_file_path("./assets/rerun/goal.glb")
            .expect("Failed to load goal model")
            .with_media_type(rerun::MediaType::glb()),
    );

    dbg.log_static(
        "field",
        &rerun::Transform3D::from_translation([0.0, 0.0, -0.01]),
    );

    dbg.log_static(
        "field/goals",
        &rerun::InstancePoses3D::new()
            .with_translations([(4.5, 0., 0.), (-4.5, 0., 0.)])
            .with_rotation_axis_angles([
                ((0., 0., 1.), 0.),
                ((0., 0., 1.), Angle::from_radians(PI).into()),
            ]),
    );
}

fn sync_cycle_number(
    mut ctx: ResMut<RerunStream>,
    cycle: Res<Cycle>,
    cycle_time: Res<CycleTime>,
    mut cycle_time_buffer: Local<Vec<(usize, Duration)>>,
) {
    if cycle_time_buffer.len() == 100 {
        let (cycles, durations): (Vec<_>, Vec<_>) = cycle_time_buffer
            .iter()
            .copied()
            .map(|(cycle, duration)| (cycle as i64, duration.as_millis() as f64))
            .unzip();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        ctx.send_columns(
            "stats/cycle_time",
            [timeline],
            rerun::Scalars::update_fields()
                .with_scalars(durations)
                .columns_of_unit_batches()
                .expect("failed to batch scalar values"),
        );
        cycle_time_buffer.clear();
    } else {
        cycle_time_buffer.push((cycle.0, cycle_time.duration));
    }

    ctx.cycle = *cycle;
}

/// A wrapper around [`rerun::RecordingStream`] that provides an infallible interface for logging data to Rerun.
///
/// Any errors that occur while logging data are logged as errors using [`tracing::error`].
#[derive(Resource, Debug, Clone)]
pub struct RerunStream {
    stream: RecordingStream,
    cycle: Cycle,
    logging_to_rrd_file: bool,
}

impl RerunStream {
    /// Initializes a new [`RerunStream`].
    ///
    /// If yggdrasil is not compiled with the `rerun` feature, this will return a
    /// [`RerunStream`] that does nothing.
    pub fn init_grpc_server(recording_name: impl AsRef<str>, rerun_host: IpAddr) -> Result<Self> {
        unsafe {
            std::env::set_var("RERUN_FLUSH_TICK_SECS", "0.15"); // 150 milliseconds
            std::env::set_var("RERUN_FLUSH_NUM_BYTES", "512000"); // 500 KiB
        }

        let rec = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
            .connect_grpc_opts(
                format!("rerun+http://{rerun_host}:{DEFAULT_SERVER_PORT}/proxy",),
                rerun::default_flush_timeout(),
            )
            .into_diagnostic()?;

        Ok(RerunStream {
            stream: rec,
            cycle: Cycle(0),
            logging_to_rrd_file: false,
        })
    }

    /// Initialize a new [`RerunStream`].
    ///
    /// The stream is stored as an rrd file at the `path` location.
    pub fn init_file_sink(
        recording_name: impl AsRef<str>,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        unsafe {
            std::env::set_var("RERUN_FLUSH_TICK_SECS", "5"); // 5 seconds
            std::env::set_var("RERUN_FLUSH_NUM_BYTES", "104857600"); // 100 MiB
        }

        let stream = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
            .save(path)
            .into_diagnostic()?;

        Ok(RerunStream {
            stream,
            cycle: Cycle(0),
            logging_to_rrd_file: true,
        })
    }

    /// Initialize a disabled [`RerunStream`].
    #[must_use]
    pub fn disabled() -> Self {
        RerunStream {
            stream: RecordingStream::disabled(),
            cycle: Cycle(0),
            logging_to_rrd_file: false,
        }
    }

    /// Whether the [`RecordingStream`] is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.stream.is_enabled()
    }

    /// Log data to Rerun.
    ///
    /// This is the main entry point for logging data to rerun. It can be used to log anything
    /// that implements the [`AsComponents`], such as any [archetype](https://docs.rs/rerun/latest/rerun/archetypes/index.html).
    ///
    /// The data will be timestamped automatically based on the current [`Cycle`], which is tracked internally.
    /// Data that needs to be logged in a specific cycle should use [`RerunStream::log_with_cycle`] instead.
    ///
    /// See [`RecordingStream::log`] for more information.
    #[inline]
    pub fn log<AS: ?Sized + AsComponents>(
        &self,
        ent_path: impl Into<EntityPath>,
        as_components: &AS,
    ) {
        if let Err(error) = self.stream.log(ent_path, as_components) {
            tracing::error!("{error}");
        }
    }

    /// Log static data to Rerun.
    ///
    /// It can be used to log anything
    /// that implements the [`AsComponents`], such as any [archetype](https://docs.rs/rerun/latest/rerun/archetypes/index.html).
    ///
    /// Static data has no time associated with it, exists on all timelines, and unconditionally shadows
    /// any temporal data of the same type.
    /// All timestamp data associated with this message will be dropped right before sending it to Rerun.
    ///
    /// See [`RecordingStream::log_static`] for more information.
    #[inline]
    pub fn log_static<AS: ?Sized + AsComponents>(
        &self,
        ent_path: impl Into<EntityPath>,
        as_components: &AS,
    ) {
        if let Err(error) = self.stream.log_static(ent_path, as_components) {
            tracing::error!("{error}");
        }
    }

    /// Log data to Rerun in the provided [`Cycle`].
    ///
    /// This is a utility function that sets the [`Cycle`] and defers all calls to log data to [`Self::log`].
    pub fn log_with_cycle<AS: ?Sized + AsComponents>(
        &self,
        ent_path: impl Into<EntityPath>,
        cycle: Cycle,
        as_components: &AS,
    ) {
        self.stream.set_time_sequence("cycle", cycle.0 as i64);
        self.log(ent_path, as_components);
        self.stream.set_time_sequence("cycle", self.cycle.0 as i64);
    }

    /// Lower-level logging API to provide data spanning multiple timepoints.
    ///
    /// Unlike the regular `log` API, which is row-oriented, this API lets you submit the data
    /// in a columnar form. The lengths of all of the [`TimeColumn`] and the component batches
    /// must match. All data that occurs at the same index across the different time and components
    /// arrays will act as a single logical row.
    ///
    /// See [`RecordingStream::send_columns`] for more information.
    #[inline]
    pub fn send_columns(
        &self,
        ent_path: impl Into<EntityPath>,
        indexes: impl IntoIterator<Item = TimeColumn>,
        columns: impl IntoIterator<Item = SerializedComponentColumn>,
    ) {
        if let Err(error) = self.stream.send_columns(ent_path, indexes, columns) {
            tracing::error!("{error}");
        }
    }

    /// Return whether the [`RerunStream`] is logging to an rrd file.
    #[must_use]
    pub fn logging_to_file_sink(&self) -> bool {
        self.logging_to_rrd_file
    }
}

/// Run condition to test whether Rerun is being logged to a [`rerun::sink::FileSink`].
#[must_use]
pub fn logging_to_file_sink(dbg: DebugContext) -> bool {
    dbg.logging_to_file_sink()
}

/// The central context used for logging debug data to [rerun](https://rerun.io).
///
/// If yggdrasil is not compiled with the `rerun` feature, all calls will result in a no-op.
#[derive(SystemParam, Deref)]
pub struct DebugContext<'w> {
    #[deref]
    rec: Res<'w, RerunStream>,
    _marker: PhantomData<&'w ()>,
}

impl DebugContext<'_> {
    /// Get a cloneable reference to the underlying [`RerunStream`] for access outside of systems (e.g. an async context).
    #[must_use]
    pub fn stream(&self) -> &RerunStream {
        &self.rec
    }
}
