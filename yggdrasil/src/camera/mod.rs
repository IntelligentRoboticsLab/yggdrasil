use miette::{IntoDiagnostic, Result};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tyr::prelude::*;

use heimdall::{Camera, YuyvImage};

/// This variable specifies how many `TopImage`'s' can be alive at the same time.
///
/// It is recommended to have at least one more buffer than is required. This way, the next frame
/// from the camera can already be stored in a buffer, reducing the latency between destructing a
/// `TopImage` and being able to fetch the newest `TopImage`.
// TODO: Replace with value from Odal.
const NUMBER_OF_TOP_CAMERA_BUFFERS: u32 = 2;

/// This variable specifies how many `BottomImage`'s' can be alive at the same time.
///
/// It is recommended to have at least one more buffer than is required. This way, the next frame
/// from the camera can already be stored in a buffer, reducing the latency between destructing a
/// `BottomImage` and being able to fetch the newest `BottomImage`.
// TODO: Replace with value from Odal.
const NUMBER_OF_BOTTOM_CAMERA_BUFFERS: u32 = 2;

/// This module captures images using the top- and bottom camera of the NAO.
///
/// The captured images are stored as image resources, which are updated whenever a newer image is
/// available from the camera.
///
/// This module provides the following resources to the application:
/// - [`TopImage`]
/// - [`BottomImage`]
pub struct CameraModule;

struct TopCamera(Arc<Mutex<Camera>>);
struct BottomCamera(Arc<Mutex<Camera>>);

impl TopCamera {
    fn new() -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(
            Camera::new_nao_top(NUMBER_OF_TOP_CAMERA_BUFFERS).into_diagnostic()?,
        ))))
    }
}

impl BottomCamera {
    fn new() -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(
            Camera::new_nao_bottom(NUMBER_OF_BOTTOM_CAMERA_BUFFERS).into_diagnostic()?,
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
    pub fn instant(&self) -> &Instant {
        &self.0 .1
    }
}

pub struct TopImage(Image);

pub struct BottomImage(Image);

impl TopImage {
    fn new(yuyv_image: YuyvImage) -> Self {
        Self(Image::new(yuyv_image))
    }

    fn take_image(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera.get_yuyv_image().into_diagnostic()?)))
    }

    pub fn image(&self) -> &Image {
        &self.0
    }
}

impl BottomImage {
    fn new(yuyv_image: YuyvImage) -> Self {
        Self(Image::new(yuyv_image))
    }

    fn take_image(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera.get_yuyv_image().into_diagnostic()?)))
    }

    pub fn image(&self) -> &Image {
        &self.0
    }
}

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let top_camera = TopCamera::new()?;
        let bottom_camera = BottomCamera::new()?;

        let top_image_resource =
            Resource::new(TopImage::take_image(&mut top_camera.0.lock().unwrap())?);
        let top_camera_resource = Resource::new(top_camera);

        let bottom_image_resource = Resource::new(BottomImage::take_image(
            &mut bottom_camera.0.lock().unwrap(),
        )?);
        let bottom_camera_resource = Resource::new(bottom_camera);

        Ok(app
            .add_resource(top_image_resource)?
            .add_resource(top_camera_resource)?
            .add_resource(bottom_image_resource)?
            .add_resource(bottom_camera_resource)?
            .add_system(camera_system))
    }
}

fn try_get_top_image(top_camera: &mut TopCamera) -> Option<TopImage> {
    let Ok(mut top_camera) = top_camera.0.try_lock() else {
        return None;
    };

    top_camera.try_get_yuyv_image().ok().map(TopImage::new)
}

fn try_get_bottom_image(top_camera: &mut BottomCamera) -> Option<BottomImage> {
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
    if let Some(new_top_image) = try_get_top_image(top_camera) {
        *top_image = new_top_image;
    }

    if let Some(new_bottom_image) = try_get_bottom_image(bottom_camera) {
        *bottom_image = new_bottom_image;
    }

    Ok(())
}
