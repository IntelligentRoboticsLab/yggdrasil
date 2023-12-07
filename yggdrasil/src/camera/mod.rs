use miette::{IntoDiagnostic, Result};
use std::sync::{Arc, Mutex};
use tyr::prelude::*;

use heimdall::{Camera, YuyvImage};

/// This variable specifies how many `TopImage`'s' can be alive at the same time.
const NUMBER_OF_TOP_CAMERA_BUFFERS: u32 = 2;

/// This variable specifies how many `BottomImage`'s' can be alive at the same time.
const NUMBER_OF_BOTTOM_CAMERA_BUFFERS: u32 = 2;

pub struct CameraModule;

struct TopCamera(Arc<Mutex<Camera>>);
struct BottomCamera(Arc<Mutex<Camera>>);

impl TopCamera {
    fn new() -> Result<Self> {
        Ok(TopCamera(Arc::new(Mutex::new(
            Camera::new_nao_top(NUMBER_OF_TOP_CAMERA_BUFFERS).into_diagnostic()?,
        ))))
    }
}

impl BottomCamera {
    fn new() -> Result<Self> {
        Ok(BottomCamera(Arc::new(Mutex::new(
            Camera::new_nao_bottom(NUMBER_OF_BOTTOM_CAMERA_BUFFERS).into_diagnostic()?,
        ))))
    }
}

pub trait Image {
    fn yuyv_image(&self) -> Arc<YuyvImage>;
}

pub struct TopImage(Arc<YuyvImage>);
pub struct BottomImage(Arc<YuyvImage>);

impl TopImage {
    fn new(camera: Arc<Mutex<Camera>>) -> Result<Self> {
        Ok(TopImage(Arc::new(
            camera.lock().unwrap().get_yuyv_image().into_diagnostic()?,
        )))
    }
}

impl BottomImage {
    fn new(camera: Arc<Mutex<Camera>>) -> Result<Self> {
        Ok(BottomImage(Arc::new(
            camera.lock().unwrap().get_yuyv_image().into_diagnostic()?,
        )))
    }
}

impl Image for TopImage {
    fn yuyv_image(&self) -> Arc<YuyvImage> {
        self.0.clone()
    }
}

impl Image for BottomImage {
    fn yuyv_image(&self) -> Arc<YuyvImage> {
        self.0.clone()
    }
}

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let top_camera = TopCamera::new()?;
        let bottom_camera = BottomCamera::new()?;

        let top_image_resource = Resource::new(TopImage::new(top_camera.0.clone())?);
        let top_camera_resource = Resource::new(top_camera);

        let bottom_image_resource = Resource::new(BottomImage::new(bottom_camera.0.clone())?);
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
    TopImage::new(top_camera)
}

async fn receive_bottom_image(bottom_camera: Arc<Mutex<Camera>>) -> Result<BottomImage> {
    BottomImage::new(bottom_camera)
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
    }
    top_image_task.try_spawn(receive_top_image(top_camera.0.clone()))?;

    if let Some(new_bottom_image) = bottom_image_task.poll() {
        *bottom_image = new_bottom_image?;
    }
    bottom_image_task.try_spawn(receive_bottom_image(bottom_camera.0.clone()))?;

    Ok(())
}
