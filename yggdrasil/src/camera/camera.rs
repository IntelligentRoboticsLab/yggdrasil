use miette::{IntoDiagnostic, Result};
use std::sync::{Arc, Mutex};
use tyr::prelude::*;

use heimdall::{Camera, YuyvImage, CAMERA_BOTTOM, CAMERA_TOP};

pub struct CameraModule;

#[derive(Clone)]
pub struct TopCamera(Arc<Mutex<Camera>>);
#[derive(Clone)]
pub struct BottomCamera(Arc<Mutex<Camera>>);

pub struct TopImage(YuyvImage);
pub struct BottomImage(YuyvImage);

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let top_camera = TopCamera(Arc::new(Mutex::new(
            Camera::new(CAMERA_TOP).into_diagnostic()?,
        )));
        let bottom_camera = BottomCamera(Arc::new(Mutex::new(
            Camera::new(CAMERA_BOTTOM).into_diagnostic()?,
        )));

        let top_image_resource = Resource::new(TopImage(
            top_camera
                .0
                .lock()
                .unwrap()
                .get_yuyv_image()
                .into_diagnostic()?,
        ));
        let top_camera_resource = Resource::new(top_camera);

        let bottom_image_resource = Resource::new(TopImage(
            bottom_camera
                .0
                .lock()
                .unwrap()
                .get_yuyv_image()
                .into_diagnostic()?,
        ));
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

async fn receive_top_image(top_camera: TopCamera) -> Result<TopImage> {
    Ok(TopImage(
        top_camera
            .0
            .lock()
            .unwrap()
            .get_yuyv_image()
            .into_diagnostic()?,
    ))
}

async fn receive_bottom_image(bottom_camera: BottomCamera) -> Result<BottomImage> {
    Ok(BottomImage(
        bottom_camera
            .0
            .lock()
            .unwrap()
            .get_yuyv_image()
            .into_diagnostic()?,
    ))
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
        top_image_task.try_spawn(receive_top_image(top_camera.clone()))?;
    }

    if let Some(new_bottom_image) = bottom_image_task.poll() {
        *bottom_image = new_bottom_image?;
        bottom_image_task.try_spawn(receive_bottom_image(bottom_camera.clone()))?;
    }

    Ok(())
}
