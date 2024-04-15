use std::{num::NonZeroU32, ops::Deref, time::Instant};

use fast_image_resize as fr;
use nalgebra::Point2;
use nidhogg::types::color;

use crate::{
    camera::{matrix::CameraMatrices, Image, TopImage},
    debug::DebugContext,
    ml::{
        util::{argmax, softmax},
        MlModel, MlTask, MlTaskResource,
    },
    prelude::*,
    vision::line::LineSegment3,
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

#[derive(Default, Debug, Clone)]
struct ProposedIntersection {
    point: Point2<f32>,
    distance: f32,
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
            .filter_map(|line| line.project_to_3d(&top_matrix).ok())
            .filter_map(|line| {
                let start_len = line.start.coords.magnitude();
                let end_len = line.end.coords.magnitude();

                let direction = (line.end - line.start).normalize();

                let start = line.start - direction / start_len;
                let end = line.end + direction / end_len;

                LineSegment3::new(start.into(), end.into())
                    .project_to_2d(&top_matrix)
                    .ok()
            })
            .collect::<Vec<_>>();

        ctx.log_lines2d_for_image(
            "top_camera/image/extended_lines",
            &extended_lines.iter().map(Into::into).collect::<Vec<_>>(),
            lines.1.clone(),
            color::u8::YELLOW,
        )?;

        let mut proposal = Vec::new();

        for i in 0..extended_lines.len() {
            let line = &extended_lines[i];
            for j in i + 1..extended_lines.len() {
                let other_line = &extended_lines[j];
                let Some(intersection) = line.intersection_point(&other_line) else {
                    continue;
                };

                let line_start = top_matrix.pixel_to_ground(line.start, 0.0);
                let line_end = top_matrix.pixel_to_ground(line.end, 0.0);

                let other_line_start = top_matrix.pixel_to_ground(other_line.start, 0.0);
                let other_line_end = top_matrix.pixel_to_ground(other_line.end, 0.0);

                match (line_start, line_end, other_line_start, other_line_end) {
                    (Ok(start), Ok(end), Ok(other_start), Ok(other_end)) => {
                        let line_direction = (end - start).normalize();
                        let other_line_direction = (other_end - other_start).normalize();

                        let distance = top_matrix
                            .pixel_to_ground(intersection, 0.0)
                            .unwrap()
                            .coords
                            .magnitude();
                        let angle = line_direction.xy().angle(&other_line_direction.xy());
                        if (angle - std::f32::consts::FRAC_PI_2).abs().to_degrees() > 10.0
                            || distance > 15.0
                        {
                            continue;
                        }

                        proposal.push(ProposedIntersection {
                            point: intersection,
                            distance,
                        });
                    }
                    _ => {}
                }
            }
        }

        // println!("Possible intersections: {:?}", possible_intersections.len());
        ctx.log_points2d_for_image_with_radius(
            "top_camera/image/intersections",
            &proposal
                .iter()
                .map(|p| (p.point.x, p.point.y))
                .collect::<Vec<_>>(),
            lines.1.clone().cycle(),
            color::u8::CYAN,
            5.0,
        )?;

        if proposal.is_empty() {
            return Ok(());
        }

        let mut intersections = Vec::new();
        let now = Instant::now();
        'outer: for i in 0..proposal.len() {
            let possible_intersection = proposal[i].clone();
            let size = (96.0 / possible_intersection.distance) as usize;
            let patch = lines.1.get_grayscale_patch(
                (
                    possible_intersection.point.x as usize,
                    possible_intersection.point.y as usize,
                ),
                size,
                size,
            );

            let patch = resize_patch(size, size, patch);
            if let Ok(()) = model.try_start_infer(&patch) {
                loop {
                    if now.elapsed().as_millis() >= 2 {
                        model.cancel();
                        break 'outer;
                    }
                    if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
                        let res = softmax(&result);
                        let max_idx = argmax(&res);

                        let class = match (max_idx, res[max_idx] >= 0.7) {
                            (0, true) => "L",
                            (1, true) => "T",
                            (2, true) => "X",
                            _ => "UNK",
                        };
                        intersections.push(format!("{}: {:.2}", class, res[max_idx]));
                        break;
                    }
                }
            }
        }

        ctx.log_boxes2d_with_class(
            "top_camera/image/field_marks",
            &proposal
                .iter()
                .map(|p| (p.point.x, p.point.y))
                .collect::<Vec<_>>(),
            &vec![(16.0, 16.0); proposal.len()],
            intersections,
            field_marks_image.0.cycle(),
        )?;
    }

    Ok(())
}

// Resize yuyv image to correct input shape
fn resize_patch(width: usize, height: usize, patch: Vec<u8>) -> Vec<f32> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(width as u32).unwrap(),
        NonZeroU32::new(height as u32).unwrap(),
        patch,
        fr::PixelType::U8,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(32).unwrap(),
        NonZeroU32::new(32).unwrap(),
        src_image.pixel_type(),
    );

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    // Remove every second y value from the yuyv image
    dst_image
        .buffer()
        .iter()
        .map(|p| *p as f32 / 255.0)
        .collect()
}

struct FieldMarkImage(Image);
