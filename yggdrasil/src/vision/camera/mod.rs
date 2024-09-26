#[cfg(not(feature = "local"))]
pub mod exposure_weights;

pub mod image;
pub mod matrix;

use crate::{nao::Cycle, prelude::*};

use bevy::prelude::*;
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};
use tasks::conditions::task_finished;

use heimdall::{
    Camera as HardwareCamera, CameraDevice, CameraLocation, CameraPosition, ExposureWeights,
};
pub use image::Image;
use matrix::CalibrationConfig;

pub const NUM_FRAMES_TO_RETAIN: usize = 3;

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
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
    pub focus_auto: bool,
    pub exposure_auto: bool,
}

/// This module captures images using the top- and bottom camera of the NAO.
///
/// The captured images are stored as image resources, which are updated whenever a newer image is
/// available from the camera.
///
/// This module provides the following resources to the application:
/// - [`TopImage`]
/// - [`BottomImage`]
#[derive(Default)]
pub struct CameraPlugin<T: CameraLocation>(PhantomData<T>);

impl<T: CameraLocation> Plugin for CameraPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, init_camera::<T>);
        app.add_systems(
            Update,
            fetch_latest_frame::<T>.run_if(task_finished::<Image<T>>),
        );

        app.add_plugins(matrix::CameraMatrixPlugin::<T>::default());
    }
}

// impl Module for CameraPlugin {
//     fn initialize(self, app: App) -> Result<App> {
//         let app = app
//             .add_startup_system(initialize_cameras)?
//             .add_system(camera_system)
//             .add_system(debug_camera_system.after(camera_system))
//             .add_task::<ComputeTask<JpegTopImage>>()?
//             .add_task::<ComputeTask<JpegBottomImage>>()?
//             .add_task::<ComputeTask<Result<ExposureWeightsCompleted>>>()?
//             .add_module(matrix::CameraMatrixModule)?;

//         #[cfg(not(feature = "local"))]
//         let app = app.add_system_chain((update_exposure_weights, set_exposure_weights));

//         Ok(app)
//     }
// }

fn setup_camera_device(settings: &CameraSettings) -> Result<CameraDevice> {
    #[cfg(feature = "local")]
    let camera_device = CameraDevice::new(&settings.path)?;
    #[cfg(not(feature = "local"))]
    let mut camera_device = CameraDevice::new(&settings.path)?;

    if settings.flip_horizontally {
        camera_device.horizontal_flip()?;
    }
    if settings.flip_vertically {
        camera_device.vertical_flip()?;
    }

    #[cfg(not(feature = "local"))]
    camera_device.set_focus_auto(settings.focus_auto)?;

    #[cfg(not(feature = "local"))]
    camera_device.set_exposure_auto(settings.exposure_auto)?;

    Ok(camera_device)
}

#[derive(Resource)]
pub struct Camera<T: CameraLocation> {
    inner: Arc<Mutex<HardwareCamera>>,
    _marker: PhantomData<T>,
}

// NOTE: This needs to be implemented manually because https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for Camera<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<T: CameraLocation + Send + Sync> Camera<T> {
    fn new(camera: HardwareCamera) -> Self {
        Self {
            inner: Arc::new(Mutex::new(camera)),
            _marker: PhantomData,
        }
    }

    fn try_fetch_image(&mut self, cycle: Cycle) -> Option<Image<T>> {
        let Ok(mut camera) = self.inner.try_lock() else {
            return None;
        };

        camera
            .try_get_yuyv_image()
            .ok()
            .map(|img| Image::new(img, cycle))
    }

    fn loop_fetch_image(&self) -> Result<Image<T>> {
        let mut camera = self.inner.lock().unwrap();

        camera
            .loop_try_get_yuyv_image()
            .into_diagnostic()
            .map(|img| Image::new(img, Cycle::default()))
    }
}

fn fetch_latest_frame<T: CameraLocation>(
    mut camera: ResMut<Camera<T>>,
    mut image: ResMut<Image<T>>,
    cycle: Res<Cycle>,
) {
    if let Some(new_image) = camera.try_fetch_image(cycle.clone()) {
        *image = new_image;
    }
}

fn init_camera<T: CameraLocation>(mut commands: Commands, config: Res<CameraConfig>) {
    let settings = match T::POSITION {
        CameraPosition::Top => &config.top,
        CameraPosition::Bottom => &config.bottom,
    };

    let camera_device = setup_camera_device(&settings).expect("failed to setup camera device");
    let hardware_camera = HardwareCamera::new(
        camera_device,
        settings.width,
        settings.height,
        settings.num_buffers,
    )
    .expect("failed to create camera hardware");

    commands.insert_resource(Camera::<T>::new(hardware_camera));
}

fn setup_exposure_weights<T: CameraLocation>(mut commands: Commands, config: Res<CameraConfig>) {
    let settings = match T::POSITION {
        CameraPosition::Top => &config.top,
        CameraPosition::Bottom => &config.bottom,
    };

    commands.insert_resource(ExposureWeights::new((settings.width, settings.height)));
}

// struct JpegTopImage(Instant);
// struct JpegBottomImage(Instant);

// #[system]
// fn debug_camera_system(
//     ctx: &DebugContext,
//     camera_matrices: &CameraMatrices,
//     bottom_image: &BottomImage,
//     bottom_task: &mut ComputeTask<JpegBottomImage>,
//     top_image: &TopImage,
//     top_task: &mut ComputeTask<JpegTopImage>,
//     robot_pose: &RobotPose,
// ) -> Result<()> {
//     let mut bottom_timestamp = Instant::now();
//     if let Some(bottom) = bottom_task.poll() {
//         bottom_timestamp = bottom.0;
//     }

//     if !bottom_task.active() && bottom_timestamp != bottom_image.timestamp {
//         let cloned = bottom_image.clone();
//         let matrix = camera_matrices.bottom.clone();
//         let ctx = ctx.clone();
//         let pose = robot_pose.clone();
//         bottom_task.try_spawn(move || {
//             log_bottom_image(ctx, cloned, &matrix, &pose).expect("Failed to log bottom image")
//         })?;
//     }

//     let mut top_timestamp = Instant::now();
//     if let Some(top) = top_task.poll() {
//         top_timestamp = top.0;
//     }

//     if !top_task.active() && top_timestamp != top_image.timestamp {
//         let cloned = top_image.clone();
//         let matrix = camera_matrices.top.clone();
//         let pose = robot_pose.clone();
//         let ctx = ctx.clone();
//         top_task.try_spawn(move || {
//             log_top_image(ctx, cloned, &matrix, &pose).expect("Failed to log top image")
//         })?;
//     }

//     Ok(())
// }

// fn log_bottom_image(
//     ctx: DebugContext,
//     bottom_image: BottomImage,
//     camera_matrix: &CameraMatrix,
//     robot_pose: &RobotPose,
// ) -> Result<JpegBottomImage> {
//     let timestamp = bottom_image.0.timestamp;
//     ctx.log_image("bottom_camera/image", bottom_image.clone().0, 20)?;
//     ctx.log_camera_matrix("bottom_camera/image", camera_matrix, &bottom_image.0)?;

//     // Transform the pinhole camera to the robot position.
//     let transform = robot_pose.as_3d() * camera_matrix.camera_to_ground;

//     ctx.log_transformation("bottom_camera/image", &transform, &bottom_image.0)?;
//     Ok(JpegBottomImage(timestamp))
// }

// fn log_top_image(
//     ctx: DebugContext,
//     top_image: TopImage,
//     camera_matrix: &CameraMatrix,
//     robot_pose: &RobotPose,
// ) -> Result<JpegTopImage> {
//     let timestamp = top_image.0.timestamp;
//     ctx.log_image("top_camera/image", top_image.clone().0, 20)?;
//     ctx.log_camera_matrix("top_camera/image", camera_matrix, &top_image.0)?;

//     // Transform the pinhole camera to the robot position.
//     let transform = robot_pose.as_3d() * camera_matrix.camera_to_ground;
//     ctx.log_transformation("top_camera/image", &transform, &top_image.0)?;
//     Ok(JpegTopImage(timestamp))
// }
