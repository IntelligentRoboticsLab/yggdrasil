use std::marker::PhantomData;

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top};
use ml::{prelude::ModelExecutor, MlModel};
use nalgebra::Point2;
use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use ml::util::{argmax, softmax};

use crate::{
    core::debug::DebugContext,
    localization::RobotPose,
    nao::Cycle,
    prelude::*,
    vision::{camera::Image, field_marks},
};

use super::line_detection::{line::LineSegment2, DetectedLines};

const IMAGE_INPUT_SIZE: usize = 32;

pub struct FieldMarksModel;

impl MlModel for FieldMarksModel {
    type Inputs = Vec<u8>;

    type Outputs = Vec<f32>;
    const ONNX_PATH: &'static str = "models/field_marks.onnx";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMarksConfig {
    pub distance_threshold: f32,
    pub angle_tolerance: f32,
    pub confidence_threshold: f32,
    pub patch_scale: f32,
    pub time_budget: usize,
}

pub struct FieldMarksPlugin<T: CameraLocation>(PhantomData<T>);

impl<T: CameraLocation> Plugin for FieldMarksPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            field_marks_system::<T>.after(super::line_detection::detect_lines_system::<T>),
        );
        // fn initialize(self, app: App) -> Result<App> {
        //     app.add_startup_system(init_field_marks)?
        //         .add_system(field_marks_system.after(super::line_detection::line_detection_system))
        //         .add_ml_task::<FieldMarksModel>()
        // }
    }
}

fn init_field_marks<T: CameraLocation>(mut commands: Commands, image: Res<Image<T>>) {
    let field_marks = FieldMarks {
        field_marks: Vec::new(),
        image: image.clone(),
    };

    commands.insert_resource(field_marks);
}

#[derive(Debug, Clone)]
pub struct IntersectionPoint {
    pub kind: IntersectionKind,
    pub point: Point2<f32>,
    pub distance_to_point: f32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum IntersectionKind {
    L,
    T,
    X,
    Unknown,
}

impl IntersectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            IntersectionKind::L => "L",
            IntersectionKind::T => "T",
            IntersectionKind::X => "X",
            IntersectionKind::Unknown => "UNK",
        }
    }
}

#[derive(Resource)]
pub struct FieldMarks<T: CameraLocation> {
    pub field_marks: Vec<IntersectionPoint>,
    pub image: Image<T>,
}

#[derive(Default, Debug, Clone)]
struct ProposedIntersection {
    point: Point2<f32>,
    distance_to_point: f32,
}

fn field_marks_system<T: CameraLocation>(
    dbg: DebugContext,
    pose: Res<RobotPose>,
    detected_lines: Query<(&Cycle, &DetectedLines), (With<T>, Added<DetectedLines>)>,
) {
    for (cycle, lines) in &detected_lines {
        let extended_lines = extend_line_segments(&lines.segments);

        dbg.log_with_cycle(
            T::make_entity_path("lines/extended"),
            *cycle,
            &rerun::LineStrips3D::update_fields().with_strips(extended_lines.iter().map(|s| {
                let point = pose.inner * *s;
                [
                    (point.start.x, point.start.y, 0.0),
                    (point.end.x, point.end.y, 0.0),
                ]
            })),
        );
    }
}

fn extend_line_segments(segments: &[LineSegment2]) -> Vec<LineSegment2> {
    segments
        .iter()
        .map(|segment| {
            let start_len = segment.start.coords.magnitude();
            let end_len = segment.end.coords.magnitude();

            let direction = (segment.end - segment.start).normalize();

            let start = segment.start - direction / start_len;
            let end = segment.end + direction / end_len;

            LineSegment2::new(start, end)
        })
        .collect::<Vec<_>>()
}

// fn field_marks_system2<T: CameraLocation>(
//     mut field_marks: ResMut<FieldMarks<T>>,
//     matrix: Res<CameraMatrix<T>>,
//     mut model: ResMut<ModelExecutor<FieldMarksModel>>,
//     config: &FieldMarksConfig,
//     ctx: &DebugContext,
// ) -> Result<()> {
//     let extended_lines = extend_lines(lines, &top_matrix);

//     ctx.log_lines2d_for_image(
//         "top_camera/image/extended_lines",
//         &extended_lines.iter().map(Into::into).collect::<Vec<_>>(),
//         &lines.1,
//         color::u8::YELLOW,
//     )?;

//     let proposals = make_proposals(&extended_lines, &top_matrix, config);
//     ctx.log_points2d_for_image_with_radius(
//         "top_camera/image/intersections",
//         &proposals
//             .iter()
//             .map(|p| (p.point.x, p.point.y))
//             .collect::<Vec<_>>(),
//         lines.1.clone().cycle(),
//         color::u8::CYAN,
//         5.0,
//     )?;

//     if proposals.is_empty() {
//         return Ok(());
//     }

//     let mut intersections = Vec::new();
//     let start_time = Instant::now();
//     'outer: for possible_intersection in proposals.iter() {
//         let size = (config.patch_scale / possible_intersection.distance_to_point) as usize;
//         let patch = lines.1.get_grayscale_patch(
//             (
//                 possible_intersection.point.x as usize,
//                 possible_intersection.point.y as usize,
//             ),
//             size,
//             size,
//         );

//         let patch = crate::core::ml::util::resize_patch(
//             (size, size),
//             (IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE),
//             patch,
//         );

//         if let Ok(()) = model.try_start_infer(&patch) {
//             loop {
//                 if start_time.elapsed().as_micros() >= config.time_budget as u128 {
//                     if let Err(e) = model.try_cancel() {
//                         tracing::warn!("Failed to cancel field mark inference: {:?}", e);
//                     }
//                     break 'outer;
//                 }

//                 if let Ok(Some(result)) = model.poll::<Vec<f32>>().transpose() {
//                     let res = softmax(&result);
//                     let max_idx = argmax(&res);

//                     let class = match (max_idx, res[max_idx] >= config.confidence_threshold) {
//                         (0, true) => IntersectionKind::L,
//                         (1, true) => IntersectionKind::T,
//                         (2, true) => IntersectionKind::X,
//                         _ => IntersectionKind::Unknown,
//                     };

//                     intersections.push(IntersectionPoint {
//                         kind: class,
//                         point: possible_intersection.point,
//                         distance_to_point: possible_intersection.distance_to_point,
//                         confidence: res[max_idx],
//                     });
//                     break;
//                 }
//             }
//         }
//     }

//     ctx.log_boxes2d_with_class(
//         "top_camera/image/field_marks",
//         &proposals
//             .iter()
//             .map(|p| (p.point.x, p.point.y))
//             .collect::<Vec<_>>(),
//         &vec![(16.0, 16.0); proposals.len()],
//         intersections
//             .iter()
//             .map(|i| i.kind.as_str().to_string())
//             .collect(),
//         field_marks_image.0.cycle(),
//     )?;

//     field_marks.image = lines.1.clone();
//     field_marks.field_marks = intersections;

//     Ok(())
// }

// fn extend_lines(lines: &TopLines, matrix: &CameraMatrix) -> Vec<LineSegment2> {
//     lines
//         .iter()
//         .filter_map(|line| line.project_to_3d(matrix).ok())
//         .filter_map(|line| {
//             let start_len = line.start.coords.magnitude();
//             let end_len = line.end.coords.magnitude();

//             let direction = (line.end - line.start).normalize();

//             let start = line.start - direction / start_len;
//             let end = line.end + direction / end_len;

//             LineSegment3::new(start, end).project_to_2d(matrix).ok()
//         })
//         .collect::<Vec<_>>()
// }

// fn make_proposals(
//     extended_lines: &[LineSegment2],
//     matrix: &CameraMatrix,
//     config: &FieldMarksConfig,
// ) -> Vec<ProposedIntersection> {
//     let mut proposals = Vec::new();
//     for i in 0..extended_lines.len() {
//         let line = &extended_lines[i];
//         for other_line in extended_lines.iter().skip(i + 1) {
//             let Some(intersection) = line.intersection_point(other_line) else {
//                 continue;
//             };

//             let line_start = matrix.pixel_to_ground(line.start, 0.0);
//             let line_end = matrix.pixel_to_ground(line.end, 0.0);

//             let other_line_start = matrix.pixel_to_ground(other_line.start, 0.0);
//             let other_line_end = matrix.pixel_to_ground(other_line.end, 0.0);

//             if let (Ok(start), Ok(end), Ok(other_start), Ok(other_end)) =
//                 (line_start, line_end, other_line_start, other_line_end)
//             {
//                 let line_direction = (end - start).normalize();
//                 let other_line_direction = (other_end - other_start).normalize();

//                 let distance = matrix
//                     .pixel_to_ground(intersection, 0.0)
//                     .unwrap()
//                     .coords
//                     .magnitude();
//                 let angle = line_direction.xy().angle(&other_line_direction.xy());
//                 let angle = (angle - std::f32::consts::FRAC_PI_2).abs().to_degrees();
//                 if angle > config.angle_tolerance || distance > config.distance_threshold {
//                     continue;
//                 }

//                 proposals.push(ProposedIntersection {
//                     point: intersection,
//                     distance_to_point: distance,
//                 });
//             }
//         }
//     }

//     proposals
// }
