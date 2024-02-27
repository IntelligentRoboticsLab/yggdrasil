#[cfg(feature = "rerun")]
use miette::IntoDiagnostic;

#[cfg(feature = "rerun")]
use std::convert::Into;

use nidhogg::types::RgbU8;
use std::net::Ipv4Addr;

use crate::{
    camera::Image,
    nao::{Cycle, RobotInfo},
    prelude::*,
};

/// A module for debugging the robot using the [rerun](https://rerun.io) viewer.
///
/// This module provides the following resources to the application:
/// - [`DebugContext`]
pub struct DebugModule;

impl Module for DebugModule {
    fn initialize(self, app: App) -> miette::Result<tyr::prelude::App> {
        Ok(app
            .add_startup_system(init_rerun)?
            .add_system(set_debug_cycle.after(crate::nao::write_hardware_info)))
    }
}

/// The central context used for logging debug data to [rerun](https://rerun.io).
///
/// If yggdrasil is not compiled with the `rerun` feature, all calls will result in a no-op.
#[derive(Clone)]
pub struct DebugContext {
    #[cfg(feature = "rerun")]
    rec: rerun::RecordingStream,
}

#[allow(unused)]
impl DebugContext {
    /// Initializes a new [`DebugContext`].
    ///
    /// If yggdrasil is not compiled with the `rerun` feature, this will return a [`DebugContext`] that
    /// does nothing.
    fn init(
        recording_name: impl AsRef<str>,
        server_address: Ipv4Addr,
        memory_limit: f32,
        ad: &AsyncDispatcher,
    ) -> Result<Self> {
        #[cfg(feature = "rerun")]
        {
            // To spawn a recording stream in serve mode, the tokio runtime needs to be in scope.
            let handle = ad.handle().clone();
            let _guard = handle.enter();

            // let rec = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
            //     .serve(
            //         &server_address.to_string(),
            //         Default::default(),
            //         Default::default(),
            //         rerun::MemoryLimit::from_fraction_of_total(memory_limit),
            //         false,
            //     )
            //     .into_diagnostic()?;

            let rec = rerun::RecordingStreamBuilder::new("yggdrasil")
                .connect_opts(
                    std::net::SocketAddr::new(
                        std::net::IpAddr::V4(Ipv4Addr::new(10, 0, 8, 38)),
                        9876,
                    ),
                    rerun::default_flush_timeout(),
                )
                .into_diagnostic()?;

            Ok(DebugContext { rec })
        }

        #[cfg(not(feature = "rerun"))]
        Ok(DebugContext {})
    }

    /// Set the current cycle index for the debug viewer.
    ///
    /// This will be used to align logs with the cycle index in the debug viewer.
    fn set_cycle(&self, cycle: &Cycle) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec.set_time_sequence("cycle", cycle.0 as i64);
        }

        Ok(())
    }

    /// Log a Yuyv encoded image to the debug viewer.
    ///
    /// The image is first converted to a jpeg encoded image.
    pub fn log_image(&self, path: impl AsRef<str>, img: Image, jpeg_quality: i32) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            let jpeg = img.yuyv_image().to_jpeg(jpeg_quality)?;
            let tensor_data =
                rerun::TensorData::from_jpeg_bytes(jpeg.to_owned()).into_diagnostic()?;
            let img = rerun::Image::try_from(tensor_data).into_diagnostic()?;
            self.rec.log(path.as_ref(), &img).into_diagnostic()?;
        }

        Ok(())
    }

    /// Set the style for a scalar series.
    ///
    /// The style will be applied to all logs of the series.
    pub fn set_scalar_series_style(
        &self,
        path: impl AsRef<str>,
        name: impl AsRef<str>,
        color: RgbU8,
        line_width: f32,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            // Use timeless logging to set the style for the entire series
            self.rec
                .log_timeless(
                    path.as_ref(),
                    &rerun::SeriesLine::new()
                        .with_color(Into::<[u8; 3]>::into(color))
                        .with_name(name.as_ref())
                        .with_width(line_width),
                )
                .into_diagnostic()?;
        }

        Ok(())
    }

    /// Log an [`f32`] scalar value to the debug viewer.
    ///
    /// The styling for the scalar series can be set using the [`DebugContext::set_scalar_series_style`] function.
    pub fn log_scalar_f32(&self, path: impl AsRef<str>, scalar: f32) -> Result<()> {
        self.log_scalar(path, scalar as f64)
    }

    // Log an [`f64`] scalar value to the debug viewer.
    //
    // The styling for the scalar series can be set using the [`DebugContext::set_scalar_series_style`] function.
    pub fn log_scalar(&self, path: impl AsRef<str>, scalar: f64) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::Scalar::new(scalar))
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn log_text(&self, path: impl AsRef<str>, text: String) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::TextLog::new(text))
                .into_diagnostic()?;
        }

        Ok(())
    }
}

#[startup_system]
fn init_rerun(storage: &mut Storage, ad: &AsyncDispatcher, robot_info: &RobotInfo) -> Result<()> {
    // Manually set the server address to the robot's IP address, instead of 0.0.0.0
    // to ensure the rerun server prints the correct connection URL on startup
    let server_address = Ipv4Addr::new(10, 0, 8, robot_info.robot_id as u8);

    // init debug context with 5% of the total memory, as cache size limit.
    let ctx = DebugContext::init("yggdrasil", server_address, 0.05, ad)?;

    storage.add_resource(Resource::new(ctx))
}

#[system]
fn set_debug_cycle(ctx: &DebugContext, cycle: &Cycle) -> Result<()> {
    ctx.set_cycle(cycle)
}
