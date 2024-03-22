pub mod matrix;

use crate::{debug::DebugContext, prelude::*};

use derive_more::{Deref, DerefMut};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use heimdall::{Camera, CameraDevice, YuyvImage};

use self::matrix::{CalibrationConfig, TopCameraMatrix};

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
            .init_resource::<TopCameraMatrix>()?
            .add_system(camera_system)
            .add_system(debug_camera_system.after(camera_system))
            .add_system(matrix::update_camera_matrix.after(camera_system))
            .add_task::<ComputeTask<JpegTopImage>>()?
            .add_task::<ComputeTask<JpegBottomImage>>()
    }
}

fn setup_camera_device(settings: &CameraSettings) -> Result<CameraDevice> {
    let camera_device = CameraDevice::new(&settings.path)?;
    if settings.flip_horizontally {
        camera_device.horizontal_flip()?;
    }
    if settings.flip_vertically {
        camera_device.vertical_flip()?;
    }

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

    fn try_fetch_image(&mut self) -> Option<Image> {
        let Ok(mut camera) = self.0.try_lock() else {
            return None;
        };

        camera.try_get_yuyv_image().ok().map(Image::new)
    }

    fn loop_fetch_image(&self) -> Result<Image> {
        let mut camera = self.0.lock().unwrap();

        camera
            .loop_try_get_yuyv_image()
            .into_diagnostic()
            .map(Image::new)
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
pub struct Image(Arc<(YuyvImage, Instant)>);

impl Image {
    fn new(yuyv_image: YuyvImage) -> Self {
        Self(Arc::new((yuyv_image, Instant::now())))
    }

    /// Return the captured image in yuyv format.
    pub fn yuyv_image(&self) -> &YuyvImage {
        &self.0 .0
    }

    /// Return the instant at which the image was captured.
    pub fn timestamp(&self) -> &Instant {
        &self.0 .1
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
) -> Result<()> {
    if let Some(new_top_image) = top_camera.try_fetch_image() {
        *top_image = TopImage::new(new_top_image);
    }

    if let Some(new_bottom_image) = bottom_camera.try_fetch_image() {
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

    storage.add_resource(top_image_resource)?;
    storage.add_resource(top_camera_resource)?;
    storage.add_resource(bottom_image_resource)?;
    storage.add_resource(bottom_camera_resource)?;

    Ok(())
}

struct JpegTopImage(Instant);
struct JpegBottomImage(Instant);

#[system]
fn debug_camera_system(
    ctx: &DebugContext,
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
        let ctx = ctx.clone();
        bottom_task.try_spawn(move || {
            log_bottom_image(ctx, cloned).expect("Failed to log bottom image")
        })?;
    }

    let mut top_timestamp = Instant::now();
    if let Some(top) = top_task.poll() {
        top_timestamp = top.0;
    }

    if !top_task.active() && &top_timestamp != top_image.timestamp() {
        let cloned = top_image.clone();
        let ctx = ctx.clone();
        top_task.try_spawn(move || log_top_image(ctx, cloned).expect("Failed to log top image"))?;
    }

    Ok(())
}

fn log_bottom_image(ctx: DebugContext, bottom_image: BottomImage) -> Result<JpegBottomImage> {
    let timestamp = bottom_image.0 .0 .1;
    ctx.log_image("bottom_camera/image", bottom_image.0, 20)?;
    Ok(JpegBottomImage(timestamp))
}

fn log_top_image(ctx: DebugContext, top_image: TopImage) -> Result<JpegTopImage> {
    let timestamp = top_image.0 .0 .1;
    ctx.log_image("top_camera/image", top_image.0, 20)?;
    Ok(JpegTopImage(timestamp))
}
