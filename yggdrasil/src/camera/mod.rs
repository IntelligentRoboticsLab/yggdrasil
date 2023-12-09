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
const NUMBER_OF_TOP_CAMERA_BUFFERS: u32 = 2;

/// This variable specifies how many `BottomImage`'s' can be alive at the same time.
///
/// It is recommended to have at least one more buffer than is required. This way, the next frame
/// from the camera can already be stored in a buffer, reducing the latency between destructing a
/// `BottomImage` and being able to fetch the newest `BottomImage`.
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
    fn new(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Arc::new((
            camera.get_yuyv_image().into_diagnostic()?,
            Instant::now(),
        ))))
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
    fn new(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera)?))
    }

    pub fn image(&self) -> &Image {
        &self.0
    }
}

impl BottomImage {
    fn new(camera: &mut Camera) -> Result<Self> {
        Ok(Self(Image::new(camera)?))
    }

    pub fn image(&self) -> &Image {
        &self.0
    }
}

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let top_camera = TopCamera::new()?;
        let bottom_camera = BottomCamera::new()?;

        let top_image_resource = Resource::new(TopImage::new(&mut top_camera.0.lock().unwrap())?);
        let top_camera_resource = Resource::new(top_camera);

        let bottom_image_resource =
            Resource::new(BottomImage::new(&mut bottom_camera.0.lock().unwrap())?);
        let bottom_camera_resource = Resource::new(bottom_camera);

        Ok(app
            .add_resource(top_image_resource)?
            .add_resource(top_camera_resource)?
            .add_resource(bottom_image_resource)?
            .add_resource(bottom_camera_resource)?
            .add_task::<AsyncTask<Result<TopImage>>>()?
            .add_task::<AsyncTask<Result<BottomImage>>>()?
            .add_system(camera_system))
    }
}

async fn receive_top_image(top_camera: Arc<Mutex<Camera>>) -> Result<TopImage> {
    TopImage::new(&mut top_camera.lock().unwrap())
}

async fn receive_bottom_image(bottom_camera: Arc<Mutex<Camera>>) -> Result<BottomImage> {
    BottomImage::new(&mut bottom_camera.lock().unwrap())
}

#[system]
fn camera_system(
    top_camera: &mut TopCamera,
    bottom_camera: &mut BottomCamera,
    top_image: &mut TopImage,
    bottom_image: &mut BottomImage,
    top_image_task: &mut AsyncTask<Result<TopImage>>,
    bottom_image_task: &mut AsyncTask<Result<BottomImage>>,
) -> Result<()> {
    if let Some(new_top_image) = top_image_task.poll() {
        *top_image = new_top_image?;
        top_image_task.try_spawn(receive_top_image(top_camera.0.clone()))?;
    } else if !top_image_task.active() {
        top_image_task.try_spawn(receive_top_image(top_camera.0.clone()))?;
    }

    if let Some(new_bottom_image) = bottom_image_task.poll() {
        *bottom_image = new_bottom_image?;
        bottom_image_task.try_spawn(receive_bottom_image(bottom_camera.0.clone()))?;
    } else if !bottom_image_task.active() {
        bottom_image_task.try_spawn(receive_bottom_image(bottom_camera.0.clone()))?;
    }

    Ok(())
}
