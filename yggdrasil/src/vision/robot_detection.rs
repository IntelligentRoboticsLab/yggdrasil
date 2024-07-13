//! Module for detecting the field boundary lines from the top camera image
//!

use std::{num::NonZeroU32, ops::Deref};

use crate::{
    core::{
        debug::DebugContext,
        ml::{self, MlModel, MlTask, MlTaskResource},
    },
    prelude::*,
    vision::camera::{Image, TopImage},
};
use fast_image_resize::{self as fr, FilterType};
use heimdall::{RgbImage, YuyvImage};
use ndarray::Axis;

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
        let img = top_image.yuyv_image();
        let rgb_image = img.to_rgb().unwrap();
        let resized_image = resize_yuyv(&rgb_image);
        let img = image::ImageBuffer::from_raw(100, 100, resized_image.clone()).unwrap();

        // img.save("image.jpg").unwrap();

        ctx.log_image_rgb("/robot_detect_input", img, &top_image.cycle())?;

        // let mean_y = 0.3335;
        // let mean_u = 0.4788;
        // let mean_v = 0.3146;

        // let std_y = 0.189_980_34;
        // let std_u = 0.163_526_65;
        // let std_v = 0.174_245_5;

        if let Ok(()) = model.try_start_infer(
            &resized_image
                .iter()
                .map(|x| *x as f32 / 255.0)
                // .enumerate()
                // .map(|(i, x)| {
                //     if i % 3 == 0 {
                //         (x - mean_y) / std_y
                //     } else if (i % 3) == 1 {
                //         (x - mean_u) / std_u
                //     } else {
                //         (x - mean_v) / std_v
                //     }
                // })
                .collect::<Vec<f32>>(),
        ) {
            // We need to keep track of the image we started the inference with
            //
            // TODO: We should find a better way to do this bundling of mltask + metadata
            *robot_detection_image = RobotDetectionImage(top_image.deref().clone());
        };
    }

    // Otherwise, poll the model for the result
    if let Some(result) = model.poll_multi::<Vec<f32>>().transpose()? {
        // println!("num results: {}", result.len());
        let box_regression = ndarray::Array2::from_shape_vec((864, 4), result[0].clone()).unwrap();
        let scores = ndarray::Array2::from_shape_vec((864, 2), result[1].clone()).unwrap();
        let features = ndarray::Array3::from_shape_vec((64, 12, 12), result[2].clone()).unwrap();

        // println!("box_regression: {box_regression:?}");
        // println!("scores: {scores:?}");
        // println!("features: {features:?}");

        let anchor_generator = detection::anchor::DefaultBoxGenerator::new(
            vec![vec![0.4, 0.5], vec![0.85]],
            0.15,
            0.9,
        );
        let box_coder = detection::box_coder::BoxCoder::new((10.0, 10.0, 5.0, 5.0));

        let decoded_boxes = box_coder.decode_single(
            box_regression,
            anchor_generator.create_boxes(
                (MODEL_INPUT_WIDTH as usize, MODEL_INPUT_HEIGHT as usize),
                features,
            ),
        );

        // println!("checking scores");
        let mut valid_scores = scores
            .axis_iter(Axis(0))
            .enumerate()
            .filter_map(|(i, s)| {
                let scores = ml::util::softmax(&[s[0], s[1]]);
                if scores[1] >= 0.1 {
                    println!("score: {}, index: {}", scores[1], i);
                    println!("bbox: {}", decoded_boxes.row(i));
                    return Some((decoded_boxes.row(i), scores[1]));
                }

                None
            })
            .collect::<Vec<_>>();

        let k = 400.min(valid_scores.len());

        // valid_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        // valid_scores.truncate(k);
        // println!("sorted scores: {valid_scores:?}");
        // let boxes = valid_scores
        //     .iter()
        //     .map(|(i, _)| {
        //         let x1 = bbox[0].clamp(0.0, 100.0);
        //         let y1 = bbox[1].clamp(0.0, 100.0);
        //         let x2 = bbox[2].clamp(0.0, 100.0);
        //         let y2 = bbox[3].clamp(0.0, 100.0);

        //         // resize boxes to original image size
        //         // let x1 = (x1 / 100.0) * 640.0;
        //         // let y1 = (y1 / 100.0) * 480.0;
        //         // let x2 = (x2 / 100.0) * 640.0;
        //         // let y2 = (y2 / 100.0) * 480.0;

        //         (x1, y1, x2, y2)
        //     })
        //     .collect::<Vec<_>>();

        // perform nms
        // let mut final_boxes = Vec::new();
        // let nms_threshold = 0.35;
        // for i in 0..boxes.len() {
        //     let mut discard = false;
        //     for j in 0..boxes.len() {
        //         if i == j {
        //             continue;
        //         }

        //         let overlap = detection::iou(&boxes[i], &boxes[j]);
        //         let score_i = valid_scores[i].1;
        //         let score_j = valid_scores[j].1;

        //         if overlap > nms_threshold {
        //             if score_j > score_i {
        //                 println!("dropped {i} due to nms with: {j}");
        //                 discard = true;
        //                 break;
        //             }
        //         }
        //     }

        //     if !discard {
        //         final_boxes.push((boxes[i], valid_scores[i].1));
        //     }
        // }

        let processed_boxes = valid_scores.iter().map(|(bbox, score)| {
            // clamp boxes to 0-100, as the model was trained on 100x100 images
            let x1 = bbox[0].clamp(0.0, 100.0);
            let y1 = bbox[1].clamp(0.0, 100.0);
            let x2 = bbox[2].clamp(0.0, 100.0);
            let y2 = bbox[3].clamp(0.0, 100.0);

            // calculate center and size
            let cx = (x1 + x2) / 2.0;
            let cy = (y1 + y2) / 2.0;
            let w = (x2 - x1) / 2.0;
            let h = (y2 - y1) / 2.0;

            (((cx, cy), (w, h)), format!("robot: {score:.4}"))
        });

        let ((centers, sizes), scores): ((Vec<_>, Vec<_>), Vec<_>) = processed_boxes.unzip();

        ctx.log_boxes2d_with_class(
            // "/top_camera/image/robots",
            "robot_detect_input/boxes",
            &centers,
            &sizes,
            scores,
            robot_detection_image.0.cycle(),
        )?;
        // println!("boxes: {decoded_boxes:?}");

        // Get the image we set when we started inference
        // let _image = robot_detection_image.0.clone();
        // println!("inference took: {:?}", image.timestamp().elapsed());

        // println!("bbox_regression: {box_regression:?}");
    }

    Ok(())
}

// Resize yuyv image to correct input shape
fn resize_yuyv(image: &RgbImage) -> Vec<u8> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(image.width() as u32).unwrap(),
        // NonZeroU32::new((image.width() / 2) as u32).unwrap(),
        NonZeroU32::new(image.height() as u32).unwrap(),
        image.to_vec(),
        fr::PixelType::U8x3,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(MODEL_INPUT_WIDTH).unwrap(),
        NonZeroU32::new(MODEL_INPUT_HEIGHT).unwrap(),
        src_image.pixel_type(),
    );

    // let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(FilterType::Bilinear));
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    // Remove every second y value from the yuyv image
    dst_image
        .buffer()
        .iter()
        .copied()
        // .enumerate()
        // .filter(|(i, _)| (i + 2) % 4 != 0)
        // .map(|(_, p)| p)
        .collect()
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
