pub mod matrix;

use crate::{core::debug::DebugContext, localization::RobotPose, nao::Cycle, prelude::*};

use derive_more::{Deref, DerefMut};
use fast_image_resize as fr;
use miette::{Context, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Instant,
};

use heimdall::{Camera, CameraDevice, CameraMatrix, ExposureWeights, YuyvImage};
use matrix::{CalibrationConfig, CameraMatrices};

#[cfg(not(feature = "local"))]
use super::field_boundary::FieldBoundary;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CameraConfig {
    pub top: CameraSettings,
    pub bottom: CameraSettings,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct CameraSettings {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub num_buffers: u32,
    pub flip_horizontally: bool,
    pub flip_vertically: bool,
    pub calibration: CalibrationConfig,
    pub focus_auto: bool,
    pub exposure_auto: bool,
}

#[derive(Debug, Clone, Default, Copy, Eq, PartialEq)]
pub enum CameraPosition {
    #[default]
    Top,
    Bottom,
}

/// This module captures images using the top- and bottom camera of the NAO.
///
/// The captured images are stored as image resources, which are updated whenever a newer image is
/// available from the camera.
///
/// This module provides the following resources to the application:
/// - [`TopImage`]
/// - [`BottomImage`]
pub struct CameraModule;

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let app = app
            .add_startup_system(initialize_cameras)?
            .add_system(camera_system)
            .add_system(debug_camera_system.after(camera_system))
            .add_task::<ComputeTask<JpegTopImage>>()?
            .add_task::<ComputeTask<JpegBottomImage>>()?
            .add_task::<ComputeTask<Result<ExposureWeightsCompleted>>>()?
            .add_module(matrix::CameraMatrixModule)?;

        #[cfg(not(feature = "local"))]
        let app = app.add_system_chain((update_exposure_weights, set_exposure_weights));

        Ok(app)
    }
}

fn setup_camera_device(settings: &CameraSettings) -> Result<CameraDevice> {
    #[cfg(feature = "local")]
    let camera_device = CameraDevice::new(&settings.path)?;
    #[cfg(not(feature = "local"))]
    let mut camera_device = CameraDevice::new(&settings.path)?;

    if settings.flip_horizontally {
        camera_device.horizontal_flip()?;
    }
    if settings.flip_vertically {
        camera_device.vertical_flip()?;
    }

    #[cfg(not(feature = "local"))]
    camera_device.set_focus_auto(settings.focus_auto)?;

    #[cfg(not(feature = "local"))]
    camera_device.set_exposure_auto(settings.exposure_auto)?;

    Ok(camera_device)
}

fn setup_camera(camera_device: CameraDevice, settings: &CameraSettings) -> Result<Camera> {
    Ok(Camera::new(
        camera_device,
        settings.width,
        settings.height,
        settings.num_buffers,
    )?)
}

pub struct YggdrasilCamera(Arc<Mutex<Camera>>);

impl YggdrasilCamera {
    fn new(camera: Camera) -> Self {
        Self(Arc::new(Mutex::new(camera)))
    }

    fn try_fetch_image(&mut self, cycle: Cycle) -> Option<Image> {
        let Ok(mut camera) = self.0.try_lock() else {
            return None;
        };

        camera
            .try_get_yuyv_image()
            .ok()
            .map(|img| Image::new(img, cycle))
    }

    fn loop_fetch_image(&self) -> Result<Image> {
        let mut camera = self.0.lock().unwrap();

        camera
            .loop_try_get_yuyv_image()
            .into_diagnostic()
            .map(|img| Image::new(img, Cycle::default()))
    }
}

#[derive(Deref, DerefMut)]
pub struct TopCamera(YggdrasilCamera);

impl TopCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        let camera_device = setup_camera_device(&config.top)?;
        let camera = setup_camera(camera_device, &config.top)?;

        Ok(Self(YggdrasilCamera::new(camera)))
    }
}

#[derive(Deref, DerefMut)]
pub struct BottomCamera(YggdrasilCamera);

impl BottomCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        let camera_device = setup_camera_device(&config.bottom)?;
        let camera = setup_camera(camera_device, &config.bottom)?;

        Ok(Self(YggdrasilCamera::new(camera)))
    }
}

#[derive(Clone, Deref)]
pub struct Image {
    #[deref]
    /// Captured image in yuyv format.
    buf: Arc<YuyvImage>,
    /// Instant at which the image was captured.
    timestamp: Instant,
    /// Return the cycle at which the image was captured.
    cycle: Cycle,
}

impl Image {
    fn new(yuyv_image: YuyvImage, cycle: Cycle) -> Self {
        Self {
            buf: Arc::new(yuyv_image),
            timestamp: Instant::now(),
            cycle,
        }
    }

    pub fn is_from_cycle(&self, cycle: Cycle) -> bool {
        self.cycle == cycle
    }

    pub fn yuyv_image(&self) -> &YuyvImage {
        &self.buf
    }

    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    pub fn cycle(&self) -> Cycle {
        self.cycle
    }

    /// Resizes the image to the given width and height using the specified algorithm.
    ///
    /// The resized image is returned as a vector of bytes, in packed YUV format.
    /// The image is converted to YUV by dropping the second y component of the YUYV format.
    pub fn resized_yuv(
        &self,
        width: u32,
        height: u32,
        algorithm: fr::ResizeAlg,
    ) -> Result<Vec<u8>> {
        let image = self.yuyv_image();

        let src_image = fr::Image::from_vec_u8(
            NonZeroU32::new((image.width() / 2) as u32).unwrap(),
            NonZeroU32::new(image.height() as u32).unwrap(),
            image.to_vec(),
            fr::PixelType::U8x4,
        )
        .into_diagnostic()
        .context("Failed to create source image for resizing!")?;

        let mut dst_image = fr::Image::new(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
            src_image.pixel_type(),
        );

        let mut resizer = fr::Resizer::new(algorithm);

        resizer
            .resize(&src_image.view(), &mut dst_image.view_mut())
            .into_diagnostic()
            .context("Failed to resize image")?;

        // Remove every second y value from the yuyv image to turn it into a packed yuv image
        Ok(dst_image
            .buffer()
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| (i + 2) % 4 != 0)
            .map(|(_, p)| p)
            .collect())
    }

    /// Get a grayscale patch from the image centered at the given point.
    /// The patch is of size `width` x `height`, and padded with zeros if the patch goes out of bounds.
    ///
    /// The grayscale values are normalized to the range [0, 1].
    pub fn get_grayscale_patch(
        &self,
        center: (usize, usize),
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let (cx, cy) = center;

        let yuyv_image = self.yuyv_image();
        let mut result = Vec::with_capacity(width * height);

        for i in 0..height {
            for j in 0..width {
                let x = cx + j - width / 2;
                let y = cy + i - height / 2;

                if x >= self.yuyv_image().width() || y >= self.yuyv_image().height() {
                    result.push(0);
                    continue;
                }

                let index = y * yuyv_image.width() + x;
                result.push(yuyv_image[index * 2]);
            }
        }

        result
    }
}

#[derive(Clone, Deref)]
pub struct TopImage(Image);

impl TopImage {
    fn new(image: Image) -> Self {
        Self(image)
    }
}

#[derive(Clone, Deref)]
pub struct BottomImage(Image);

impl BottomImage {
    fn new(image: Image) -> Self {
        Self(image)
    }
}

#[system]
pub fn camera_system(
    top_camera: &mut TopCamera,
    bottom_camera: &mut BottomCamera,
    top_image: &mut TopImage,
    bottom_image: &mut BottomImage,
    cycle: &Cycle,
) -> Result<()> {
    if let Some(new_top_image) = top_camera.try_fetch_image(*cycle) {
        *top_image = TopImage::new(new_top_image);
    }

    if let Some(new_bottom_image) = bottom_camera.try_fetch_image(*cycle) {
        *bottom_image = BottomImage::new(new_bottom_image);
    }

    Ok(())
}

#[startup_system]
fn initialize_cameras(storage: &mut Storage, config: &CameraConfig) -> Result<()> {
    let top_camera = TopCamera::new(config)?;
    let bottom_camera = BottomCamera::new(config)?;

    let width = top_camera.0 .0.lock().unwrap().width() as u32;
    let height = top_camera.0 .0.lock().unwrap().height() as u32;

    let top_image_resource = Resource::new(TopImage::new(top_camera.loop_fetch_image()?));
    let top_camera_resource = Resource::new(top_camera);

    let bottom_image_resource = Resource::new(BottomImage::new(bottom_camera.loop_fetch_image()?));
    let bottom_camera_resource = Resource::new(bottom_camera);

    let exposure_weights = Resource::new(ExposureWeights::new((width, height)));

    storage.add_resource(top_image_resource)?;
    storage.add_resource(top_camera_resource)?;
    storage.add_resource(bottom_image_resource)?;
    storage.add_resource(bottom_camera_resource)?;
    storage.add_resource(exposure_weights)?;

    Ok(())
}

struct JpegTopImage(Instant);
struct JpegBottomImage(Instant);

#[system]
fn debug_camera_system(
    ctx: &DebugContext,
    camera_matrices: &CameraMatrices,
    bottom_image: &BottomImage,
    bottom_task: &mut ComputeTask<JpegBottomImage>,
    top_image: &TopImage,
    top_task: &mut ComputeTask<JpegTopImage>,
    robot_pose: &RobotPose,
) -> Result<()> {
    let mut bottom_timestamp = Instant::now();
    if let Some(bottom) = bottom_task.poll() {
        bottom_timestamp = bottom.0;
    }

    if !bottom_task.active() && bottom_timestamp != bottom_image.timestamp {
        let cloned = bottom_image.clone();
        let matrix = camera_matrices.bottom.clone();
        let ctx = ctx.clone();
        let pose = robot_pose.clone();
        bottom_task.try_spawn(move || {
            log_bottom_image(ctx, cloned, &matrix, &pose).expect("Failed to log bottom image")
        })?;
    }

    let mut top_timestamp = Instant::now();
    if let Some(top) = top_task.poll() {
        top_timestamp = top.0;
    }

    if !top_task.active() && top_timestamp != top_image.timestamp {
        let cloned = top_image.clone();
        let matrix = camera_matrices.top.clone();
        let pose = robot_pose.clone();
        let ctx = ctx.clone();
        top_task.try_spawn(move || {
            log_top_image(ctx, cloned, &matrix, &pose).expect("Failed to log top image")
        })?;
    }

    Ok(())
}

fn log_bottom_image(
    ctx: DebugContext,
    bottom_image: BottomImage,
    camera_matrix: &CameraMatrix,
    robot_pose: &RobotPose,
) -> Result<JpegBottomImage> {
    let timestamp = bottom_image.0.timestamp;
    ctx.log_image("bottom_camera/image", bottom_image.clone().0, 20)?;
    ctx.log_camera_matrix("bottom_camera/image", camera_matrix, &bottom_image.0)?;

    // Transform the pinhole camera to the robot position.
    let transform = robot_pose.as_3d() * camera_matrix.camera_to_ground;

    ctx.log_transformation("bottom_camera/image", &transform, &bottom_image.0)?;
    Ok(JpegBottomImage(timestamp))
}

fn log_top_image(
    ctx: DebugContext,
    top_image: TopImage,
    camera_matrix: &CameraMatrix,
    robot_pose: &RobotPose,
) -> Result<JpegTopImage> {
    let timestamp = top_image.0.timestamp;
    ctx.log_image("top_camera/image", top_image.clone().0, 20)?;
    ctx.log_camera_matrix("top_camera/image", camera_matrix, &top_image.0)?;

    // Transform the pinhole camera to the robot position.
    let transform = robot_pose.as_3d() * camera_matrix.camera_to_ground;
    ctx.log_transformation("top_camera/image", &transform, &top_image.0)?;
    Ok(JpegTopImage(timestamp))
}

#[cfg(not(feature = "local"))]
const SAMPLES_PER_COLUMN: usize = 4;

#[cfg(not(feature = "local"))]
const ABOVE_FIELD_WEIGHT: u8 = 0;

#[cfg(not(feature = "local"))]
const BELOW_FIELD_WEIGHT: u8 = 15;

#[cfg(not(feature = "local"))]
const MIN_BOTTOM_ROW_WEIGHT: u8 = 10;

#[cfg(not(feature = "local"))]
const WEIGHT_SLOPE: f32 = (BELOW_FIELD_WEIGHT - ABOVE_FIELD_WEIGHT) as f32;

#[cfg(not(feature = "local"))]
#[system]
fn update_exposure_weights(
    exposure_weights: &mut ExposureWeights,
    field_boundary: &FieldBoundary,
) -> Result<()> {
    let (width, height) = exposure_weights.top.window_size();
    let (column_width, row_height) = (width / 4, height / 4);

    let mut weights = [0; 16];

    for column_index in 0..4 {
        let column_start = column_index * column_width;
        let column_end = column_start + column_width;

        let samples = (column_start..column_end)
            .step_by(column_width as usize / SAMPLES_PER_COLUMN)
            .map(|x| field_boundary.height_at_pixel(x as f32));

        let n = samples.len() as f32;
        let field_height = (samples.sum::<f32>() / n) as u32;

        for row_index in 0..4 {
            let row_start = row_index * row_height;
            let row_end = row_start + row_height;

            let weight_index = row_index * 4 + column_index;

            weights[weight_index as usize] = if row_end < field_height {
                ABOVE_FIELD_WEIGHT
            } else if row_start > field_height {
                BELOW_FIELD_WEIGHT
            } else {
                let fract = (field_height - row_start) as f32 / row_height as f32;

                ((ABOVE_FIELD_WEIGHT as f32 + WEIGHT_SLOPE * fract) as u8)
                    .clamp(ABOVE_FIELD_WEIGHT, BELOW_FIELD_WEIGHT)
            }
        }
    }

    for weight in weights.iter_mut().skip(12) {
        *weight = (*weight).max(MIN_BOTTOM_ROW_WEIGHT);
    }

    exposure_weights.top.update(weights);
    Ok(())
}

struct ExposureWeightsCompleted;

#[cfg(not(feature = "local"))]
#[system]
fn set_exposure_weights(
    exposure_weights: &ExposureWeights,
    top_camera: &TopCamera,
    bottom_camera: &BottomCamera,
    task: &mut ComputeTask<Result<ExposureWeightsCompleted>>,
) -> Result<()> {
    let exposure_weights = exposure_weights.clone();
    let top_camera = top_camera.0 .0.clone();
    let bottom_camera = bottom_camera.0 .0.clone();

    if let Some(result) = task.poll() {
        result?;
    }

    let _ = task.try_spawn(move || {
        if let Ok(top_camera) = top_camera.lock() {
            top_camera
                .camera_device()
                .set_auto_exposure_weights(&exposure_weights.top)?;
        }

        if let Ok(bottom_camera) = bottom_camera.lock() {
            bottom_camera
                .camera_device()
                .set_auto_exposure_weights(&exposure_weights.bottom)?;
        }

        Ok(ExposureWeightsCompleted)
    });

    Ok(())
}
