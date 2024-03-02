#[cfg(feature = "rerun")]
use miette::IntoDiagnostic;
use nidhogg::types::RgbU8;
use std::{net::Ipv4Addr, time::Instant};

use crate::{camera::Image, nao::Cycle, prelude::*};

#[cfg(not(feature = "rerun"))]
use crate::{config::yggdrasil::YggdrasilConfig, nao::RobotInfo};

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
    #[cfg(feature = "rerun")]
    start_time: std::time::Instant,
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

            let rec = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
                .serve(
                    &server_address.to_string(),
                    Default::default(),
                    Default::default(),
                    rerun::MemoryLimit::from_fraction_of_total(memory_limit),
                    false,
                )
                .into_diagnostic()?;

            Ok(DebugContext {
                rec,
                start_time: Instant::now(),
            })
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
            self.rec.set_time_seconds(
                "image",
                img.timestamp()
                    .duration_since(self.start_time)
                    .as_secs_f64(),
            );
            let jpeg = img.yuyv_image().to_jpeg(jpeg_quality)?;
            let tensor_data =
                rerun::TensorData::from_jpeg_bytes(jpeg.to_owned()).into_diagnostic()?;
            let img = rerun::Image::try_from(tensor_data).into_diagnostic()?;

            self.rec.log(path.as_ref(), &img).into_diagnostic()?;
            self.rec.disable_timeline("image");
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

    pub fn log_point2d(&self, path: impl AsRef<str>, x: f32, y: f32) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::Points2D::new([(x, y)]))
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn log_points2d_for_image(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        img: Image,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            let image_timestamp = img.timestamp();
            self.rec.set_time_seconds(
                "image",
                image_timestamp
                    .duration_since(self.start_time)
                    .as_secs_f64(),
            );
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points2D::new(points).with_colors(vec![
                        rerun::Color::from_rgb(
                            255, 0, 0,
                        );
                        points.len()
                    ]),
                )
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn log_point2d(&self, path: impl AsRef<str>, x: f32, y: f32) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::Points2D::new([(x, y)]))
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn log_points2d_for_image(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        img: Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            let image_timestamp = img.timestamp();
            self.rec.set_time_seconds(
                "image",
                image_timestamp
                    .duration_since(self.start_time)
                    .as_secs_f64(),
            );
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points2D::new(points).with_colors(vec![
                        rerun::Color::from_rgb(
                            color.red,
                            color.green,
                            color.blue,
                        );
                        points.len()
                    ]),
                )
                .into_diagnostic()?;
        }

        Ok(())
    }

    pub fn log_lines2d_for_image(
        &self,
        path: impl AsRef<str>,
        lines: &[[(f32, f32); 2]],
        img: Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            let image_timestamp = img.timestamp();
            self.rec.set_time_seconds(
                "image",
                image_timestamp
                    .duration_since(self.start_time)
                    .as_secs_f64(),
            );
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::LineStrips2D::new(lines).with_colors(vec![
                        rerun::Color::from_rgb(
                            color.red,
                            color.green,
                            color.blue,
                        );
                        lines.len()
                    ]),
                )
                .into_diagnostic()?;
        }

        Ok(())
    }
}

#[startup_system]
fn init_rerun(
    storage: &mut Storage,
    ad: &AsyncDispatcher,
    #[cfg(not(feature = "local"))] robot_info: &RobotInfo,
    #[cfg(not(feature = "local"))] yggdrasil_config: &YggdrasilConfig,
) -> Result<()> {
    #[cfg(feature = "local")]
    let server_address = Ipv4Addr::LOCALHOST;
    // Manually set the server address to the robot's IP address, instead of 0.0.0.0
    // to ensure the rerun server prints the correct connection URL on startup
    #[cfg(not(feature = "local"))]
    let server_address = Ipv4Addr::new(
        10,
        0,
        yggdrasil_config.game_controller.team_number,
        robot_info.robot_id as u8,
    );

    // init debug context with 5% of the total memory, as cache size limit.
    let ctx = DebugContext::init("yggdrasil", server_address, 0.05, ad)?;

    storage.add_resource(Resource::new(ctx))
}

#[system]
fn set_debug_cycle(ctx: &DebugContext, cycle: &Cycle) -> Result<()> {
    ctx.set_cycle(cycle)
}
