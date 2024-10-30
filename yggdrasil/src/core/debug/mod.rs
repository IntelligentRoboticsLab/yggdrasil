use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use miette::IntoDiagnostic;
use rerun::components::Scalar;
use rerun::{AsComponents, ComponentBatch, EntityPath, RecordingStream, TimeColumn};
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{convert::Into, net::SocketAddr};
use std::{marker::PhantomData, net::IpAddr};

use crate::{
    nao::{Cycle, CycleTime},
    prelude::*,
};

const DEFAULT_STORAGE_PATH: &str = "/mnt/usb";
const STORAGE_PATH_ENV_NAME: &str = "RERUN_STORAGE_PATH";
const DATE_TIME_FORMAT: &str = "%Y-%m-%d:%H-%M-%S";

/// Plugin that adds debugging tools for the robot using the [rerun](https://rerun.io) viewer.
///
/// This introduces a [`DebugContext`] [`SystemParam`], which can be used
/// for common debugging tasks.
pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init_rerun, setup_spl_field).chain())
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

fn init_rerun(mut commands: Commands) {
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
            "Rerun logging to {}",
            output_rrd_file_path.as_path().display()
        );
        RerunStream::init_file_store("yggdrasil", output_rrd_file_path)
            .expect("failed to initialize rerun::RecordingStream")
    } else if let Some(address) = server_address {
        RerunStream::init("yggdrasil", address)
            .expect("failed to initialize rerun::RecordingStream")
    } else {
        tracing::warn!("`RERUN_HOST` not set, rerun debugging is disabled");
        RerunStream::disabled()
    };

    commands.insert_resource(rec);
}

fn setup_spl_field(dbg: DebugContext) {
    dbg.log_static(
        "field/mesh",
        &rerun::Asset3D::from_file("./assets/rerun/spl_field.glb")
            .expect("Failed to load field model")
            .with_media_type(rerun::MediaType::glb()),
    );

    dbg.log_static(
        "field/mesh",
        &rerun::Transform3D::from_translation([0.0, 0.0, -0.05]),
    );

    dbg.log_static("field/mesh", &rerun::ViewCoordinates::FLU);
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

        let scalar_data: Vec<Scalar> = durations.into_iter().map(Into::into).collect();

        let timeline = TimeColumn::new_sequence("cycle", cycles);
        ctx.send_columns("stats/cycle_time", [timeline], [&scalar_data as _]);
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
}

impl RerunStream {
    /// Initializes a new [`RerunStream`].
    ///
    /// If yggdrasil is not compiled with the `rerun` feature, this will return a
    /// [`RerunStream`] that does nothing.
    pub fn init(recording_name: impl AsRef<str>, rerun_host: IpAddr) -> Result<Self> {
        let rec = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
            .connect_opts(
                SocketAddr::new(rerun_host, rerun::default_server_addr().port()),
                rerun::default_flush_timeout(),
            )
            .into_diagnostic()?;

        Ok(RerunStream {
            stream: rec,
            cycle: Cycle(0),
        })
    }

    /// Initialize a new [`RerunStream`].
    ///
    /// The stream is stored as an rrd file at the `path` location.
    pub fn init_file_store(
        recording_name: impl AsRef<str>,
        path: impl Into<PathBuf>,
    ) -> Result<Self> {
        let stream = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
            .save(path)
            .into_diagnostic()?;

        Ok(RerunStream {
            stream,
            cycle: Cycle(0),
        })
    }

    /// Initialize a disabled [`RerunStream`].
    #[must_use]
    pub fn disabled() -> Self {
        RerunStream {
            stream: RecordingStream::disabled(),
            cycle: Cycle(0),
        }
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
    pub fn log(&self, ent_path: impl Into<EntityPath>, arch: &impl AsComponents) {
        if let Err(error) = self.stream.log(ent_path, arch) {
            error!("{error}");
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
    pub fn log_static(&self, ent_path: impl Into<EntityPath>, arch: &impl AsComponents) {
        if let Err(error) = self.stream.log_static(ent_path, arch) {
            error!("{error}");
        }
    }

    /// Log data to Rerun in the provided [`Cycle`].
    ///
    /// This is a utility function that sets the [`Cycle`] and defers all calls to log data to [`Self::log`].
    pub fn log_with_cycle(
        &self,
        ent_path: impl Into<EntityPath>,
        cycle: Cycle,
        arch: &impl AsComponents,
    ) {
        self.stream.set_time_sequence("cycle", cycle.0 as i64);
        self.log(ent_path, arch);
        self.stream.set_time_sequence("cycle", self.cycle.0 as i64);
    }

    /// Logs a set of [`ComponentBatch`]es into Rerun.
    ///
    /// If `static_` is set to `true`, all timestamp data associated with this message will be
    /// dropped right before sending it to Rerun.
    /// Static data has no time associated with it, exists on all timelines, and unconditionally shadows
    /// any temporal data of the same type.
    ///
    /// See [`RecordingStream::log_component_batches`] for more information.
    pub fn log_component_batches<'a>(
        &self,
        ent_path: impl Into<EntityPath>,
        static_: bool,
        comp_batches: impl IntoIterator<Item = &'a dyn ComponentBatch>,
    ) {
        if let Err(error) = self
            .stream
            .log_component_batches(ent_path, static_, comp_batches)
        {
            error!("{error}");
        }
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
    pub fn send_columns<'a>(
        &self,
        ent_path: impl Into<EntityPath>,
        timelines: impl IntoIterator<Item = TimeColumn>,
        components: impl IntoIterator<Item = &'a dyn ComponentBatch>,
    ) {
        if let Err(error) = self.stream.send_columns(ent_path, timelines, components) {
            error!("{error}");
        }
    }
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
