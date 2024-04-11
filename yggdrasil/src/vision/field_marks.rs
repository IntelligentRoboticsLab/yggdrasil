use std::{num::NonZeroU32, ops::Deref, time::Instant};

use fast_image_resize as fr;
use fr::CropBox;
use nalgebra::Point2;
use ndarray::{Array3, ArrayBase, ArrayView3, Dimension, IntoDimension, Ix3, OwnedRepr};
use nidhogg::types::color;
use tracing::field;

use crate::{
    camera::{matrix::CameraMatrices, Image, TopImage},
    debug::DebugContext,
    ml::{data_type::Output, MlModel, MlTask, MlTaskResource},
    prelude::*,
    vision::line::LineSegment,
};

use super::line_detection::TopLines;

pub struct FieldMarksModel;

impl MlModel for FieldMarksModel {
    type InputType = f32;
    type OutputType = f32;
    const ONNX_PATH: &'static str = "models/field_marks.onnx";
}

pub struct FieldMarksModule;

impl Module for FieldMarksModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(init_field_marks)?
            .add_system(field_marks_system.after(super::line_detection::line_detection_system))
            .add_ml_task::<FieldMarksModel>()
    }
}

#[startup_system]
fn init_field_marks(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let field_mark_image = FieldMarkImage(top_image.deref().clone());

    // Initialize the field boundary with a single line at the top of the image
    let field_marks = FieldMarks {
        field_marks: Vec::new(),
        image: top_image.deref().clone(),
    };

    storage.add_resource(Resource::new(field_mark_image))?;
    storage.add_resource(Resource::new(field_marks))?;

    Ok(())
}

pub enum FieldMark {
    X(Point2<f32>),
    L(Point2<f32>),
    T(Point2<f32>),
}

pub struct FieldMarks {
    pub field_marks: Vec<FieldMark>,
    pub image: Image,
}

#[system]
fn field_marks_system(
    lines: &TopLines,
    field_marks: &mut FieldMarks,
    matrix: &CameraMatrices,
    model: &mut MlTask<FieldMarksModel>,
    field_marks_image: &mut FieldMarkImage,
    ctx: &DebugContext,
) -> Result<()> {
    if field_marks_image.0.cycle() != lines.1.cycle() && !model.active() {
        // tracing::info!("got new lines, finding field marks!");
        *field_marks_image = FieldMarkImage(lines.1.clone());
        let top_matrix = matrix.top.clone();

        let extended_lines = lines
            .iter()
            .filter_map(|line| {
                let start = top_matrix.pixel_to_ground(line.start, 0.0);
                let end = top_matrix.pixel_to_ground(line.end, 0.0);

                if let Ok(start) = start {
                    if let Ok(end) = end {
                        // extend line away from camera if it's facing away
                        let direction = (end - start).normalize();
                        let start = start - direction * 0.25;
                        let end = end + direction * 0.25;

                        return Some(LineSegment {
                            start: top_matrix
                                .ground_to_pixel(start)
                                .expect("failed to project ground to camera for start"),
                            end: top_matrix
                                .ground_to_pixel(end)
                                .expect("failed to project ground to camera for end"),
                        });
                    }
                }

                None
            })
            .collect::<Vec<_>>();

        ctx.log_lines2d_for_image(
            "top_camera/image/extended_lines",
            &extended_lines.iter().map(|s| s.into()).collect::<Vec<_>>(),
            lines.1.clone(),
            color::u8::YELLOW,
        )?;

        let mut possible_intersections = Vec::new();
        extended_lines.iter().for_each(|line| {
            extended_lines.iter().for_each(|other_line| {
                if let Some(intersection) = line.intersection_point(&other_line) {
                    if line.angle_between(other_line) >= std::f32::consts::FRAC_PI_4 {
                        possible_intersections.push(intersection);
                    }
                }
            });
        });

        // println!("Possible intersections: {:?}", possible_intersections.len());
        ctx.log_points2d_for_image_with_radius(
            "top_camera/image/intersections",
            &possible_intersections
                .iter()
                .map(|p| (p.x, p.y))
                .collect::<Vec<_>>(),
            lines.1.clone().cycle(),
            color::u8::CYAN,
            5.0,
        )?;

        if possible_intersections.is_empty() {
            return Ok(());
        }

        let intersection = possible_intersections[0];
        let patch = get_patch(intersection, field_marks_image.0.clone());
        ctx.log_ndarray_image(
            "top_camera/patch",
            lines.1.clone().cycle(),
            Array3::from_shape_vec((32, 32, 1), patch.clone()).unwrap(),
        )?;
        // tracing::info!("starting field marks!");
        if let Ok(()) = model.try_start_infer(&patch) {
            // We need to keep track of the image we started the inference with
            //
            // TODO: We should find a better way to do this bundling of mltask + metadata
            let now = Instant::now();
            // tracing::info!("Started field marks inference");
            loop {
                if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                    tracing::info!("inference took: {:?}", now.elapsed(),);
                    let res = softmax(&result);
                    let max_idx = argmax(&res);
                    // tracing::info!("got result: {:?}, argmax: {}", res, max_idx);

                    let class = match (max_idx, res[max_idx] >= 0.8) {
                        (0, true) => "L",
                        (1, true) => "T",
                        (2, true) => "X",
                        _ => "UNK",
                    };

                    ctx.log_box2d_class(
                        "top_camera/image/patch",
                        (intersection[0], intersection[1]),
                        (16.0, 16.0),
                        class,
                        field_marks_image.0.cycle(),
                    )?;
                    break;
                }
            }
        };
    }

    Ok(())
}

fn argmax(v: &Vec<f32>) -> usize {
    let mut max_idx = 0;
    let mut max_val = v[0];

    for (i, &val) in v.iter().enumerate() {
        if val > max_val {
            max_val = val;
            max_idx = i;
        }
    }

    max_idx
}

fn softmax(v: &Vec<f32>) -> Vec<f32> {
    let mut sum = 0.0;
    let mut result = Vec::new();

    for &x in v {
        let e = x.exp();
        sum += e;
        result.push(e);
    }

    for x in result.iter_mut() {
        *x /= sum;
    }

    result
}

fn get_patch(point: Point2<f32>, image: Image) -> Vec<f32> {
    let x = point.x as usize;
    let y = point.y as usize;

    let yuyv_image = image.yuyv_image();
    let mut result = Vec::new();

    for i in 0..32 {
        for j in 0..32 {
            let x = x + j - 16;
            let y = y + i - 16;

            if x >= yuyv_image.width() || y >= yuyv_image.height() {
                result.push(0.0);
                continue;
            }

            let index = y * yuyv_image.width() + x;
            result.push(yuyv_image[index * 2] as f32 / 255.0);
        }
    }

    result
}

struct FieldMarkImage(Image);
