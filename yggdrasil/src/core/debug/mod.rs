#[cfg(feature = "rerun")]
use std::{convert::Into, net::SocketAddr};

use heimdall::CameraMatrix;
#[cfg(feature = "rerun")]
use miette::IntoDiagnostic;

use nalgebra::Isometry3;
use nidhogg::types::RgbU8;

use std::net::IpAddr;

#[cfg(feature = "rerun")]
use heimdall::YuvPlanarImage;

use crate::{
    nao::{Cycle, CycleTime},
    prelude::*,
    vision::camera::Image,
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
            .add_staged_system(SystemStage::Init, set_debug_cycle))
    }
}

/// The central context used for logging debug data to [rerun](https://rerun.io).
///
/// If yggdrasil is not compiled with the `rerun` feature, all calls will result in a no-op.
#[derive(Clone)]
pub struct DebugContext {
    #[cfg(feature = "rerun")]
    rec: rerun::RecordingStream,
    current_cycle: Cycle,
}

#[allow(unused)]
impl DebugContext {
    /// Initializes a new [`DebugContext`].
    ///
    /// If yggdrasil is not compiled with the `rerun` feature, this will return a [`DebugContext`] that
    /// does nothing.
    pub fn init(recording_name: impl AsRef<str>, rerun_host: IpAddr) -> Result<Self> {
        #[cfg(feature = "rerun")]
        {
            let rec = rerun::RecordingStreamBuilder::new(recording_name.as_ref())
                .connect_opts(
                    SocketAddr::new(rerun_host, rerun::default_server_addr().port()),
                    rerun::default_flush_timeout(),
                )
                .into_diagnostic()?;

            Ok(DebugContext {
                rec,
                current_cycle: Cycle(0),
            })
        }

        #[cfg(not(feature = "rerun"))]
        Ok(DebugContext {
            current_cycle: Cycle(0),
        })
    }

    /// Set the current cycle index for the debug viewer.
    ///
    /// This will be used to align logs with the cycle index in the debug viewer.
    fn set_cycle(&self, cycle: &Cycle) {
        #[cfg(feature = "rerun")]
        {
            self.rec.set_time_sequence("cycle", cycle.0 as i64);
        }
    }

    /// Disable the "cycle" timeline for the current thread.
    fn clear_cycle(&self) {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .set_time_sequence("cycle", self.current_cycle.0 as i64);
        }
    }

    /// Log a Yuyv encoded image to the debug viewer.
    ///
    /// The image is first converted to a jpeg encoded image.
    pub fn log_image(&self, path: impl AsRef<str>, img: Image, jpeg_quality: i32) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&img.cycle());
            let yuv_planar_image = YuvPlanarImage::from_yuyv(img.yuyv_image());
            let jpeg = yuv_planar_image.to_jpeg(jpeg_quality)?;
            let tensor_data =
                rerun::TensorData::from_jpeg_bytes(jpeg.to_owned()).into_diagnostic()?;
            let img = rerun::Image::try_from(tensor_data).into_diagnostic()?;

            self.rec.log(path.as_ref(), &img).into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log an RGB image to the debug viewer.
    pub fn log_image_rgb(
        &self,
        path: impl AsRef<str>,
        img: image::RgbImage,
        cycle: &Cycle,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(cycle);
            let tensor_data = rerun::TensorData::from_image(img).into_diagnostic()?;
            let img = rerun::Image::try_from(tensor_data).into_diagnostic()?;

            self.rec.log(path.as_ref(), &img).into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    pub fn log_patch(
        &self,
        path: impl AsRef<str>,
        image: Cycle,
        img: ndarray::Array3<f32>,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image);
            let img = rerun::Image::try_from(img).into_diagnostic()?;
            self.rec.log(path.as_ref(), &img).into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    pub fn log_boxes_2d(
        &self,
        path: impl AsRef<str>,
        centers: &[(f32, f32)],
        sizes: &[(f32, f32)],
        image: &Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());

            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Boxes2D::from_centers_and_sizes(centers, sizes)
                        .with_colors(vec![(Into::<[u8; 3]>::into(color)); centers.len()]),
                )
                .into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    pub fn log_boxes2d_with_class(
        &self,
        path: impl AsRef<str>,
        centers: &[(f32, f32)],
        half_sizes: &[(f32, f32)],
        labels: Vec<String>,
        cycle: Cycle,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&cycle);
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Boxes2D::from_centers_and_half_sizes(centers, half_sizes)
                        .with_labels(labels),
                )
                .into_diagnostic()?;

            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a camera matrix to the debug viewer.
    ///
    /// The camera matrix is logged as a pinhole camera, without any transforms applied.
    pub fn log_camera_matrix(
        &self,
        path: impl AsRef<str>,
        matrix: &CameraMatrix,
        image: &Image,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
            let pinhole = rerun::Pinhole::from_focal_length_and_resolution(
                [matrix.focal_lengths.x, matrix.focal_lengths.y],
                [
                    image.yuyv_image().width() as f32,
                    image.yuyv_image().height() as f32,
                ],
            )
            .with_camera_xyz(rerun::components::ViewCoordinates::FLU)
            .with_image_plane_distance(1.0);
            self.rec.log(path.as_ref(), &pinhole).into_diagnostic()?;
            self.clear_cycle();
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
                .log_static(
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

    /// Log a text message to the debug viewer.
    pub fn log_text(&self, path: impl AsRef<str>, text: String) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::TextLog::new(text))
                .into_diagnostic()?;
        }

        Ok(())
    }

    /// Log a set of 2D points to the debug viewer.
    pub fn log_points_2d(
        &self,
        path: impl AsRef<str>,
        points: impl IntoIterator<Item = (f32, f32)>,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::Points2D::new(points))
                .into_diagnostic()?;
        }

        Ok(())
    }

    /// Log a set of 2D points to the debug viewer, using the timestamp of the provided image.
    pub fn log_points2d_for_image(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        image: &Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
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
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 2D points to the debug viewer, using the timestamp of the provided image.
    pub fn log_points2d_for_image_with_colors(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        image: &Image,
        color: &[RgbU8],
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points2D::new(points).with_colors(
                        color
                            .iter()
                            .map(|c| rerun::Color::from_rgb(c.red, c.green, c.blue)),
                    ),
                )
                .into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 2D points to the debug viewer, using the timestamp of the provided image.
    pub fn log_points2d_for_image_with_radius(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        cycle: Cycle,
        color: RgbU8,
        radius: f32,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&cycle);
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points2D::new(points)
                        .with_colors(vec![
                            rerun::Color::from_rgb(
                                color.red,
                                color.green,
                                color.blue,
                            );
                            points.len()
                        ])
                        .with_radii(vec![radius; points.len()]),
                )
                .into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 2D points to the debug viewer, using the timestamp of the provided image.
    pub fn log_points2d_for_image_with_radii(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32)],
        cycle: Cycle,
        color: RgbU8,
        radii: Vec<f32>,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&cycle);
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points2D::new(points)
                        .with_colors(vec![
                            rerun::Color::from_rgb(
                                color.red,
                                color.green,
                                color.blue,
                            );
                            points.len()
                        ])
                        .with_radii(radii),
                )
                .into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 2D lines to the debug viewer, using the timestamp of the provided image.
    pub fn log_lines2d_for_image_with_colors(
        &self,
        path: impl AsRef<str>,
        lines: &[[(f32, f32); 2]],
        image: &Image,
        colors: &[RgbU8],
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::LineStrips2D::new(lines).with_colors(
                        colors
                            .iter()
                            .map(|c| rerun::Color::from_rgb(c.red, c.green, c.blue)),
                    ),
                )
                .into_diagnostic()?;

            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 2D lines to the debug viewer, using the timestamp of the provided image.
    pub fn log_lines2d_for_image(
        &self,
        path: impl AsRef<str>,
        lines: &[[(f32, f32); 2]],
        image: &Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
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

            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a set of 3D points to the debug viewer.
    pub fn log_points_3d(
        &self,
        path: impl AsRef<str>,
        points: impl IntoIterator<Item = (f32, f32, f32)>,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(path.as_ref(), &rerun::Points3D::new(points))
                .into_diagnostic()?;
        }
        Ok(())
    }

    /// Log a set of 3D points to the debug viewer, using the provided color and radius.
    pub fn log_points_3d_with_color_and_radius(
        &self,
        path: impl AsRef<str>,
        points: &[(f32, f32, f32)],
        color: RgbU8,
        radius: f32,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            let color = rerun::Color::from_rgb(color.red, color.green, color.blue);
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Points3D::new(points)
                        .with_radii(vec![radius; points.len()])
                        .with_colors(vec![color; points.len()]),
                )
                .into_diagnostic()?;
        }
        Ok(())
    }

    /// Log a set of 3D arrows to the debug viewer.
    pub fn log_arrows3d_with_color(
        &self,
        path: impl AsRef<str>,
        vectors: &[(f32, f32, f32)],
        origins: &[(f32, f32, f32)],
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::Arrows3D::from_vectors(vectors)
                        .with_origins(origins)
                        .with_colors(vec![
                            rerun::Color::from_rgb(
                                color.red,
                                color.green,
                                color.blue
                            );
                            vectors.len()
                        ]),
                )
                .into_diagnostic()?;
        }
        Ok(())
    }

    /// Log a set of 3D lines to the debug viewer, using the timestamp of the provided image.
    pub fn log_lines3d_for_image(
        &self,
        path: impl AsRef<str>,
        lines: &[[(f32, f32, f32); 2]],
        image: &Image,
        color: RgbU8,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());
            self.rec
                .log(
                    path.as_ref(),
                    &rerun::LineStrips3D::new(lines).with_colors(vec![
                        rerun::Color::from_rgb(
                            color.red,
                            color.green,
                            color.blue,
                        );
                        lines.len()
                    ]),
                )
                .into_diagnostic()?;
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a transformation to the entities at the provided path.
    pub fn log_transformation(
        &self,
        path: impl AsRef<str>,
        transform: &Isometry3<f32>,
        image: &Image,
    ) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.set_cycle(&image.cycle());

            let translation = transform.translation;
            let rotation = transform.rotation.coords;

            self.rec.log(
                path.as_ref(),
                &rerun::Transform3D::from_translation_rotation(
                    (translation.x, translation.y, translation.z),
                    rerun::Quaternion([rotation.x, rotation.y, rotation.z, rotation.w]),
                ),
            );
            self.clear_cycle();
        }

        Ok(())
    }

    /// Log a timeless robot view coordinate system to the debug viewer.
    /// This sets the x-axis to the front of the robot, the y-axis to the left, and the z-axis up.
    pub fn log_robot_viewcoordinates(&self, path: impl AsRef<str>) -> Result<()> {
        #[cfg(feature = "rerun")]
        {
            self.rec
                .log_static(path.as_ref(), &rerun::ViewCoordinates::FLU)
                .into_diagnostic()?;
        }

        Ok(())
    }
}

#[startup_system]
fn init_rerun(storage: &mut Storage) -> Result<()> {
    #[cfg(any(feature = "local", not(feature = "rerun")))]
    let server_address = IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED);
    // Manually set the server address to the robot's IP address, instead of 0.0.0.0
    // to ensure the rerun server prints the correct connection URL on startup
    #[cfg(all(not(feature = "local"), feature = "rerun"))]
    let server_address = {
        let host = std::env::var("RERUN_HOST").into_diagnostic()?;

        std::str::FromStr::from_str(host.as_str()).into_diagnostic()?
    };

    let ctx = DebugContext::init("yggdrasil", server_address)?;

    #[cfg(feature = "rerun")]
    {
        ctx.rec
            .log_static(
                "field/mesh",
                &rerun::Asset3D::from_file("./assets/rerun/spl_field.glb")
                    .expect("Failed to load field model")
                    .with_transform(
                        rerun::Transform3D::from_translation([0.0, 0.0, -0.05])
                            .transform
                            .0,
                    ),
            )
            .into_diagnostic()?;

        ctx.log_robot_viewcoordinates("/field/mesh")?;
    }

    storage.add_resource(Resource::new(ctx))
}

#[system]
fn set_debug_cycle(ctx: &mut DebugContext, cycle: &Cycle, cycle_time: &CycleTime) -> Result<()> {
    ctx.set_cycle(cycle);
    ctx.log_scalar_f32("cycle_time", cycle_time.duration.as_millis() as f32)?;
    ctx.current_cycle = *cycle;
    Ok(())
}
