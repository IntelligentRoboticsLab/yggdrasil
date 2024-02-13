use crate::prelude::*;

use miette::IntoDiagnostic;
use rerun::{EntityPath, RecordingStream, TensorData};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
    time::Instant,
};

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

        app.add_resource(top_image_resource)?
            .add_resource(top_camera_resource)?
            .add_resource(bottom_image_resource)?
            .add_resource(bottom_camera_resource)?
            .add_system(camera_system)
            .add_task::<ComputeTask<JpegTopImage>>()?
            .add_task::<ComputeTask<JpegBottomImage>>()
    }
}

struct TopCamera(Arc<Mutex<Camera>>);

impl TopCamera {
    fn new() -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(
            Camera::new_nao_top(NUMBER_OF_TOP_CAMERA_BUFFERS).into_diagnostic()?,
        ))))
    }
}

struct BottomCamera(Arc<Mutex<Camera>>);

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
    rec: &RecordingStream,
    top_camera_debug: &mut ComputeTask<JpegTopImage>,
    bottom_camera_debug: &mut ComputeTask<JpegBottomImage>,
) -> Result<()> {
    if let Some(new_top_image) = try_fetch_top_image(top_camera) {
        *top_image = new_top_image;
        let cloned = top_image.0.clone();
        let rec = rec.clone();
        if !top_camera_debug.active() {
            top_camera_debug.try_spawn(|| {
                log_jpeg_image(cloned, rec, "top_image").expect("failed to log top image");
                JpegTopImage
            })?;
        }
    }

    if let Some(new_bottom_image) = try_fetch_bottom_image(bottom_camera) {
        *bottom_image = new_bottom_image;

        let cloned = bottom_image.0.clone();
        let rec = rec.clone();
        if !bottom_camera_debug.active() {
            bottom_camera_debug.try_spawn(|| {
                log_jpeg_image(cloned, rec, "bottom_image").expect("failed to log top image");
                JpegBottomImage
            })?;
        }
    }

    Ok(())
}

pub struct JpegTopImage;
pub struct JpegBottomImage;

fn log_jpeg_image(image: Image, rec: RecordingStream, path: impl Into<EntityPath>) -> Result<()> {
    let yuyv_image = image.yuyv_image();
    let mut jpeg = Vec::new();

    yuyv_image.to_jpeg(&mut jpeg)?;
    let tensor_data = TensorData::from_jpeg_bytes(jpeg).into_diagnostic()?;
    let img = rerun::Image::try_from(tensor_data).into_diagnostic()?;
    rec.log(path, &img).into_diagnostic()?;

    Ok(())
}
