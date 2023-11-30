use miette::{IntoDiagnostic, Result};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tyr::prelude::*;

use heimdall::{Camera, YuyvImage, CAMERA_BOTTOM, CAMERA_TOP};

pub struct CameraModule;

struct TopCamera(Arc<Mutex<Camera>>);
struct BottomCamera(Arc<Mutex<Camera>>);

pub struct TopImage(pub Arc<YuyvImage>);
pub struct BottomImage(pub Arc<YuyvImage>);

pub struct TopImageInstant(Instant, u32);

impl Module for CameraModule {
    fn initialize(self, app: App) -> Result<App> {
        let top_camera = TopCamera(Arc::new(Mutex::new(
            Camera::new(CAMERA_TOP).into_diagnostic()?,
        )));
        let bottom_camera = BottomCamera(Arc::new(Mutex::new(
            Camera::new(CAMERA_BOTTOM).into_diagnostic()?,
        )));

        let top_image_resource = Resource::new(TopImage(Arc::new(
            top_camera
                .0
                .lock()
                .unwrap()
                .get_yuyv_image()
                .into_diagnostic()?,
        )));
        let top_camera_resource = Resource::new(top_camera);

        let bottom_image_resource = Resource::new(TopImage(Arc::new(
            bottom_camera
                .0
                .lock()
                .unwrap()
                .get_yuyv_image()
                .into_diagnostic()?,
        )));
        let bottom_camera_resource = Resource::new(bottom_camera);

        Ok(app
            .add_resource(top_image_resource)?
            .add_resource(top_camera_resource)?
            .add_resource(bottom_image_resource)?
            .add_resource(bottom_camera_resource)?
            .add_task::<AsyncTask<Result<TopImage>>>()?
            .add_task::<AsyncTask<Result<BottomImage>>>()?
            .add_resource(Resource::new(TopImageInstant(Instant::now(), 0)))?
            .add_system(camera_system))
    }
}

async fn receive_top_image(top_camera: Arc<Mutex<Camera>>) -> Result<TopImage> {
    Ok(TopImage(Arc::new(
        top_camera
            .lock()
            .unwrap()
            .get_yuyv_image()
            .into_diagnostic()?,
    )))
}

async fn receive_bottom_image(bottom_camera: Arc<Mutex<Camera>>) -> Result<BottomImage> {
    Ok(BottomImage(Arc::new(
        bottom_camera
            .lock()
            .unwrap()
            .get_yuyv_image()
            .into_diagnostic()?,
    )))
}

#[system]
fn camera_system(
    top_camera: &mut TopCamera,
    bottom_camera: &mut BottomCamera,
    top_image: &mut TopImage,
    bottom_image: &mut BottomImage,
    top_image_task: &mut AsyncTask<Result<TopImage>>,
    bottom_image_task: &mut AsyncTask<Result<BottomImage>>,
    top_image_instant: &mut TopImageInstant,
) -> Result<()> {
    eprintln!("HERE");
    if top_image_instant.0.duration_since(Instant::now()) > Duration::from_secs(2) {
        top_image
            .0
            .store_jpeg(&format!("/home/nao/image_{}.jpg", top_image_instant.1))
            .into_diagnostic()?;
    }

    if let Some(new_top_image) = top_image_task.poll() {
        *top_image = new_top_image?;
        top_image_task.try_spawn(receive_top_image(top_camera.0.clone()))?;
    }

    if let Some(new_bottom_image) = bottom_image_task.poll() {
        *bottom_image = new_bottom_image?;
        bottom_image_task.try_spawn(receive_bottom_image(bottom_camera.0.clone()))?;
    }

    Ok(())
}
