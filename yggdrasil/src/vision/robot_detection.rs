//! Module for detecting the field boundary lines from the top camera image
//!

use std::{num::NonZeroU32, ops::Deref};

use crate::{
    core::{
        debug::DebugContext,
        ml::{MlModel, MlTask, MlTaskResource},
    },
    prelude::*,
    vision::camera::{Image, TopImage},
};
use fast_image_resize as fr;
use heimdall::RgbImage;
use ndarray::Array3;

const MODEL_INPUT_WIDTH: u32 = 100;
const MODEL_INPUT_HEIGHT: u32 = 100;

pub struct RobotDetectionModule;

impl Module for RobotDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_ml_task::<RobotDetectionModel>()?
            .add_startup_system(init_robot_detection)?
            .add_system(detect_robots))
    }
}

#[derive(Debug, Clone)]
pub struct DetectedRobot {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A fitted field boundary from a given image
#[derive(Clone)]
pub struct RobotDetectionData {
    /// The fitted field boundary lines
    pub robots: Vec<DetectedRobot>,
    /// The image the boundary was predicted from
    pub image: Image,
}

/// For keeping track of the image that a robot detection was made from
struct RobotDetectionImage(Image);

#[system]
fn detect_robots(
    model: &mut MlTask<RobotDetectionModel>,
    robot_detection_image: &mut RobotDetectionImage,
    _robots: &mut RobotDetectionData,
    ctx: &mut DebugContext,
    top_image: &TopImage,
) -> Result<()> {
    // Start a new inference if the image has changed
    // TODO: Some kind of callback/event system would be nice to avoid doing the timestamp comparison everywhere
    if robot_detection_image.0.timestamp() != top_image.timestamp() && !model.active() {
        let rgb = top_image.yuyv_image().to_rgb().unwrap();
        let resized_image = resize_yuyv(&rgb);

        ctx.log_image_rgb(
            "/robot_detect_input",
            image::ImageBuffer::from_raw(
                MODEL_INPUT_WIDTH,
                MODEL_INPUT_HEIGHT,
                resized_image.clone(),
            )
            .unwrap(),
            &top_image.cycle(),
        )?;

        if let Ok(()) = model.try_start_infer(
            &resized_image
                .iter()
                .map(|x| *x as f32 / 255.0)
                .collect::<Vec<f32>>(),
        ) {
            // We need to keep track of the image we started the inference with
            //
            // TODO: We should find a better way to do this bundling of mltask + metadata
            *robot_detection_image = RobotDetectionImage(top_image.deref().clone());
        };
    }

    // Otherwise, poll the model for the result
    if let Some(result) = model.poll::<Vec<f32>>().transpose()? {
        println!("num results: {}", result.len());
        let box_regression = ndarray::Array2::from_shape_vec((1152, 4), result).unwrap();

        let anchor_generator = detection::anchor::DefaultBoxGenerator::new(
            vec![vec![0.4, 0.5, 0.6], vec![0.6, 0.85, 1.11]],
            0.15,
            0.9,
        );
        let box_coder = detection::box_coder::BoxCoder::new((10.0, 10.0, 5.0, 5.0));

        let _decoded_boxes = box_coder.decode_single(
            box_regression,
            anchor_generator.create_boxes(
                (MODEL_INPUT_WIDTH as usize, MODEL_INPUT_HEIGHT as usize),
                Array3::zeros((1, 1152, 4)),
            ),
        );

        // println!("boxes: {decoded_boxes:?}");

        // Get the image we set when we started inference
        let _image = robot_detection_image.0.clone();
        // println!("inference took: {:?}", image.timestamp().elapsed());

        // println!("bbox_regression: {box_regression:?}");
    }

    Ok(())
}

// Resize yuyv image to correct input shape
fn resize_yuyv(rgb_image: &RgbImage) -> Vec<u8> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(rgb_image.width() as u32).unwrap(),
        NonZeroU32::new(rgb_image.height() as u32).unwrap(),
        rgb_image.to_vec(),
        fr::PixelType::U8x3,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(MODEL_INPUT_WIDTH).unwrap(),
        NonZeroU32::new(MODEL_INPUT_HEIGHT).unwrap(),
        src_image.pixel_type(),
    );

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    // Remove every second y value from the yuyv image
    dst_image.buffer().to_vec()
}

/// A model implementing the network from B-Human their [Deep Field Boundary](https://b-human.de/downloads/publications/2022/DeepFieldBoundary.pdf) paper
pub struct RobotDetectionModel;

impl MlModel for RobotDetectionModel {
    type InputType = f32;
    type OutputType = f32;
    const ONNX_PATH: &'static str = "models/robot_detection.onnx";
}

#[startup_system]
fn init_robot_detection(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let robot_detection_image = RobotDetectionImage(top_image.deref().clone());

    // Initialize the field boundary with a single line at the top of the image
    let detected_robots = RobotDetectionData {
        robots: vec![],
        image: top_image.deref().clone(),
    };

    storage.add_resource(Resource::new(robot_detection_image))?;
    storage.add_resource(Resource::new(detected_robots))?;

    Ok(())
}
