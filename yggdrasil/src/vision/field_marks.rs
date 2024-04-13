use std::{num::NonZeroU32, ops::Deref, time::Instant};

use fast_image_resize as fr;
use fr::CropBox;
use nalgebra::{iter, Point2};
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
        *field_marks_image = FieldMarkImage(lines.1.clone());
        let top_matrix = matrix.top.clone();

        let extended_lines = lines
            .iter()
            .filter_map(|line| {
                let start = top_matrix.pixel_to_ground(line.start, 0.0);
                let end = top_matrix.pixel_to_ground(line.end, 0.0);

                if let Ok(start) = start {
                    if let Ok(end) = end {
                        let start_distance_to_camera = start.coords.norm();
                        let end_distance_to_camera = end.coords.norm();
                        // extend line away from camera if it's facing away
                        let direction = (end - start).normalize();
                        let start = start - direction * (0.1 * start_distance_to_camera);
                        let end = end + direction * (0.1 * end_distance_to_camera);

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
        let mut angles = Vec::new();
        extended_lines.iter().enumerate().for_each(|(i, line)| {
            for j in i + 1..extended_lines.len() {
                let other_line = &extended_lines[j];
                if let Some(intersection) = line.intersection_point(&other_line) {
                    let intersection_to_v1 = intersection - line.start;
                    let intersection_to_v2 = intersection - other_line.start;
                    let angle = intersection_to_v1.angle(&intersection_to_v2);

                    // if angle <= 30.0f32.to_radians() {
                    // continue;
                    // }
                    possible_intersections.push((intersection.x, intersection.y));
                    angles.push(angle);
                }
            }
        });
        // extended_lines.iter().for_each(|line| {
        //     extended_lines.iter().for_each(|other_line| {
        //         if let Some(intersection) = line.intersection_point(&other_line) {
        //             // if line.angle_between(other_line) >= std::f32::consts::FRAC_PI_4 {
        //             possible_intersections.push((intersection.x, intersection.y));
        //             angles.push(line.angle_between(other_line));
        //             // }
        //         }
        //     });
        // });

        // println!("Possible intersections: {:?}", possible_intersections.len());
        ctx.log_points2d_for_image_with_radius(
            "top_camera/image/intersections",
            &possible_intersections,
            lines.1.clone().cycle(),
            color::u8::CYAN,
            5.0,
        )?;

        if possible_intersections.is_empty() {
            return Ok(());
        }

        let intersection = possible_intersections[0];

        // ctx.log_ndarray_image(
        //     "top_camera/patch",
        //     lines.1.clone().cycle(),
        //     Array3::from_shape_vec((32, 32, 1), patch.clone()).unwrap(),
        // )?;

        let mut intersections = Vec::new();
        let now = Instant::now();
        for i in 0..possible_intersections.len() {
            let possible_intersection = possible_intersections[i];
            let patch = lines.1.get_grayscale_patch(
                (
                    possible_intersection.0 as usize,
                    possible_intersection.1 as usize,
                ),
                32,
                32,
            );
            if let Ok(()) = model.try_start_infer(&patch) {
                // We need to keep track of the image we started the inference with
                //
                // TODO: We should find a better way to do this bundling of mltask + metadata

                loop {
                    if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                        let res = softmax(&result);
                        let max_idx = argmax(&res);

                        let class = match (max_idx, res[max_idx] >= 0.8) {
                            (0, true) => "L",
                            (1, true) => "T",
                            (2, true) => "X",
                            _ => "UNK",
                        };
                        intersections.push(format!("{}: {:.2}", class, angles[i].to_degrees()));
                        break;
                    }
                }
            }
        }
        tracing::info!(
            "total inference for {} field-marks took: {:?}",
            intersections.len(),
            now.elapsed()
        );

        ctx.log_boxes2d_with_class(
            "top_camera/image/field_marks",
            &possible_intersections,
            &vec![(16.0, 16.0); possible_intersections.len()],
            intersections,
            field_marks_image.0.cycle(),
        )?;
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

struct FieldMarkImage(Image);
