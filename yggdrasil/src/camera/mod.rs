use crate::{debug::DebugContext, prelude::*};

use miette::IntoDiagnostic;
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
const NUMBER_OF_TOP_CAMERA_BUFFERS: u32 = 3;

/// This variable specifies how many `BottomImage`'s' can be alive at the same time.
///
/// It is recommended to have at least one more buffer than is required. This way, the next frame
/// from the camera can already be stored in a buffer, reducing the latency between destructing a
/// `BottomImage` and being able to fetch the newest `BottomImage`.
// TODO: Replace with value from Odal.
const NUMBER_OF_BOTTOM_CAMERA_BUFFERS: u32 = 3;

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
            .add_system(debug_camera_system.after(camera_system))
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
) -> Result<()> {
    if let Some(new_top_image) = try_fetch_top_image(top_camera) {
        *top_image = new_top_image;
    }

    if let Some(new_bottom_image) = try_fetch_bottom_image(bottom_camera) {
        *bottom_image = new_bottom_image;
    }

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
