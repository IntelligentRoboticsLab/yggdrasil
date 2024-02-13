use crate::{debug::DebugContext, prelude::*};

use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
    time::Instant,
};

use heimdall::{Camera, YuyvImage};

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CameraConfig {
    pub num_top_buffers: u32,
    pub num_bottom_buffers: u32,
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
        Ok(app
            .add_startup_system(initialize_cameras)?
            .add_system(camera_system))
    }
}

struct TopCamera(Arc<Mutex<Camera>>);

impl TopCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(
            Camera::new_nao_top(config.num_top_buffers).into_diagnostic()?,
        ))))
    }
}

struct BottomCamera(Arc<Mutex<Camera>>);

impl BottomCamera {
    fn new(config: &CameraConfig) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(
            Camera::new_nao_bottom(config.num_bottom_buffers).into_diagnostic()?,
        ))))
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

#[derive(Clone)]
pub struct TopImage(Image);

impl TopImage {
    fn new(yuyv_image: YuyvImage) -> Self {
        Self(Image::new(yuyv_image))
    }

    fn take_image(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera.get_yuyv_image().into_diagnostic()?)))
    }
}

impl Deref for TopImage {
    type Target = Image;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct BottomImage(Image);

impl BottomImage {
    fn new(yuyv_image: YuyvImage) -> Self {
        Self(Image::new(yuyv_image))
    }

    fn take_image(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera.get_yuyv_image().into_diagnostic()?)))
    }
}

impl Deref for BottomImage {
    type Target = Image;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

fn try_fetch_top_image(top_camera: &mut TopCamera) -> Option<TopImage> {
    let Ok(mut top_camera) = top_camera.0.try_lock() else {
        return None;
    };

    top_camera.try_get_yuyv_image().ok().map(TopImage::new)
}

fn try_fetch_bottom_image(top_camera: &mut BottomCamera) -> Option<BottomImage> {
    let Ok(mut bottom_camera) = top_camera.0.try_lock() else {
        return None;
    };

    bottom_camera
        .try_get_yuyv_image()
        .ok()
        .map(BottomImage::new)
}

#[system]
fn camera_system(
    top_camera: &mut TopCamera,
    bottom_camera: &mut BottomCamera,
    top_image: &mut TopImage,
    bottom_image: &mut BottomImage,
) -> Result<()> {
    if let Some(new_top_image) = try_fetch_top_image(top_camera) {
        *top_image = new_top_image;
    }

    if let Some(new_bottom_image) = try_fetch_bottom_image(bottom_camera) {
        *bottom_image = new_bottom_image;
    }

    Ok(())
}

#[startup_system]
fn initialize_cameras(storage: &mut Storage, config: &CameraConfig) -> Result<()> {
    let top_camera = TopCamera::new(config)?;
    let bottom_camera = BottomCamera::new(config)?;

    let top_image_resource =
        Resource::new(TopImage::take_image(&mut top_camera.0.lock().unwrap())?);
    let top_camera_resource = Resource::new(top_camera);

    let bottom_image_resource = Resource::new(BottomImage::take_image(
        &mut bottom_camera.0.lock().unwrap(),
    )?);
    let bottom_camera_resource = Resource::new(bottom_camera);

    storage.add_resource(top_image_resource)?;
    storage.add_resource(top_camera_resource)?;
    storage.add_resource(bottom_image_resource)?;
    storage.add_resource(bottom_camera_resource)?;

    Ok(())
}
