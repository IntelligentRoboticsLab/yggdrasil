pub mod matrix;

use crate::{debug::DebugContext, nao::Cycle, prelude::*};

use derive_more::{Deref, DerefMut};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use heimdall::{Camera, CameraDevice, CameraMatrix, ExposureWeights, YuyvImage};
use matrix::{CalibrationConfig, CameraMatrices};

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
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CameraPosition {
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
        app.add_startup_system(initialize_cameras)?
            .add_system(camera_system)
            .add_system(debug_camera_system.after(camera_system))
            .add_system(set_exposure_weights)
            .add_task::<ComputeTask<JpegTopImage>>()?
            .add_task::<ComputeTask<JpegBottomImage>>()?
            .add_module(matrix::CameraMatrixModule)
    }
}

fn setup_camera_device(settings: &CameraSettings) -> Result<CameraDevice> {
    let mut camera_device = CameraDevice::new(&settings.path)?;
    if settings.flip_horizontally {
        camera_device.horizontal_flip()?;
    }
    if settings.flip_vertically {
        camera_device.vertical_flip()?;
    }

    camera_device.set_focus_auto(false)?;
    camera_device.set_exposure_auto(true)?;

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

struct YggdrasilCamera(Arc<Mutex<Camera>>);

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
struct TopCamera(YggdrasilCamera);

impl TopCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        let camera_device = setup_camera_device(&config.top)?;
        let camera = setup_camera(camera_device, &config.top)?;

        Ok(Self(YggdrasilCamera::new(camera)))
    }
}

#[derive(Deref, DerefMut)]
struct BottomCamera(YggdrasilCamera);

impl BottomCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        let camera_device = setup_camera_device(&config.bottom)?;
        let camera = setup_camera(camera_device, &config.bottom)?;

        Ok(Self(YggdrasilCamera::new(camera)))
    }
}

#[derive(Clone)]
pub struct Image(Arc<(YuyvImage, Instant, Cycle)>);

impl Image {
    fn new(yuyv_image: YuyvImage, cycle: Cycle) -> Self {
        Self(Arc::new((yuyv_image, Instant::now(), cycle)))
    }

    /// Return the captured image in yuyv format.
    pub fn yuyv_image(&self) -> &YuyvImage {
        &self.0 .0
    }

    /// Return the instant at which the image was captured.
    pub fn timestamp(&self) -> &Instant {
        &self.0 .1
    }

    /// Return the cycle at which the image was captured.
    pub fn cycle(&self) -> Cycle {
        self.0 .2
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
fn camera_system(
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

    let top_image_resource = Resource::new(TopImage::new(top_camera.loop_fetch_image()?));
    let top_camera_resource = Resource::new(top_camera);

    let bottom_image_resource = Resource::new(BottomImage::new(bottom_camera.loop_fetch_image()?));
    let bottom_camera_resource = Resource::new(bottom_camera);

    let exposure_weights =
        Resource::new(ExposureWeights::new((config.top.width, config.top.height)));

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
) -> Result<()> {
    let mut bottom_timestamp = Instant::now();
    if let Some(bottom) = bottom_task.poll() {
        bottom_timestamp = bottom.0;
    }

    if !bottom_task.active() && &bottom_timestamp != bottom_image.timestamp() {
        let cloned = bottom_image.clone();
        let matrix = camera_matrices.bottom.clone();
        let ctx = ctx.clone();
        bottom_task.try_spawn(move || {
            log_bottom_image(ctx, cloned, &matrix).expect("Failed to log bottom image")
        })?;
    }

    let mut top_timestamp = Instant::now();
    if let Some(top) = top_task.poll() {
        top_timestamp = top.0;
    }

    if !top_task.active() && &top_timestamp != top_image.timestamp() {
        let cloned = top_image.clone();
        let matrix = camera_matrices.top.clone();
        let ctx = ctx.clone();
        top_task.try_spawn(move || {
            log_top_image(ctx, cloned, &matrix).expect("Failed to log top image")
        })?;
    }

    Ok(())
}

fn log_bottom_image(
    ctx: DebugContext,
    bottom_image: BottomImage,
    camera_matrix: &CameraMatrix,
) -> Result<JpegBottomImage> {
    let timestamp = bottom_image.0 .0 .1;
    ctx.log_image("bottom_camera/image", bottom_image.clone().0, 20)?;
    ctx.log_camera_matrix("bottom_camera/image", camera_matrix, bottom_image.clone().0)?;

    // For now, let's also transform the pinhole camera to the ground frame.
    ctx.log_transformation(
        "bottom_camera/image",
        &camera_matrix.camera_to_ground,
        bottom_image.clone().0,
    )?;
    Ok(JpegBottomImage(timestamp))
}

fn log_top_image(
    ctx: DebugContext,
    top_image: TopImage,
    camera_matrix: &CameraMatrix,
) -> Result<JpegTopImage> {
    let timestamp = top_image.0 .0 .1;
    ctx.log_image("top_camera/image", top_image.clone().0, 20)?;
    ctx.log_camera_matrix("top_camera/image", camera_matrix, top_image.clone().0)?;

    // For now, let's also transform the pinhole camera to the ground frame.
    ctx.log_transformation(
        "top_camera/image",
        &camera_matrix.camera_to_ground,
        top_image.clone().0,
    )?;
    Ok(JpegTopImage(timestamp))
}

#[system]
fn set_exposure_weights(
    exposure_weights: &mut ExposureWeights,
    top_camera: &TopCamera,
    bottom_camera: &BottomCamera,
) -> Result<()> {
    if let Ok(top_camera) = top_camera.0 .0.try_lock() {
        top_camera
            .get_camera_device()
            .set_auto_exposure_weights(exposure_weights.top.encode())?;
    }

    if let Ok(bottom_camera) = bottom_camera.0 .0.try_lock() {
        bottom_camera
            .get_camera_device()
            .set_auto_exposure_weights(exposure_weights.bottom.encode())?;
    }

    Ok(())
}
