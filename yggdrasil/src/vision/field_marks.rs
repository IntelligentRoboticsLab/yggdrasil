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

#[derive(Default)]
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

#[derive(Debug, Clone, Default)]
pub enum IntersectionKind {
    L,
    T,
    X,
    #[default]
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
    kind: IntersectionKind,
}

fn field_marks_system<T: CameraLocation>(
    dbg: DebugContext,
    pose: Res<RobotPose>,
    detected_lines: Query<(&Cycle, &DetectedLines), (With<T>, Added<DetectedLines>)>,
) {
    for (cycle, lines) in &detected_lines {
        let extended_lines = extend_line_segments(&lines.segments);
        let proposals = make_proposals(&extended_lines);

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

        dbg.log_with_cycle(
            T::make_entity_path("lines/intersections"),
            *cycle,
            &rerun::Points3D::update_fields()
                .with_positions(proposals.iter().map(|s| {
                    let point = pose.inner * s.point;
                    (point.x, point.y, 0.0)
                }))
                .with_labels(proposals.iter().map(|s| format!("{:?}", s.kind))),
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

fn make_proposals(extended_lines: &[LineSegment2]) -> Vec<ProposedIntersection> {
    let mut proposals = Vec::new();
    for i in 0..extended_lines.len() {
        let line = &extended_lines[i];
        for other_line in extended_lines.iter().skip(i + 1) {
            let Some(intersection) = line.intersection_point(other_line) else {
                continue;
            };

            let (start, end) = (line.start, line.end);
            let (other_start, other_end) = (other_line.start, other_line.end);

            let line_direction = (end - start).normalize();
            let other_line_direction = (other_end - other_start).normalize();

            let angle = line_direction.xy().angle(&other_line_direction.xy());
            let angle = (angle - std::f32::consts::FRAC_PI_2).abs().to_degrees();
            let distance = intersection.coords.magnitude();
            if angle > 10.0 || distance > 15.0 {
                continue;
            }

            proposals.push(ProposedIntersection {
                point: intersection,
                distance_to_point: distance,
                kind: classify_intersection(line, other_line, &intersection),
            });
        }
    }

    proposals
}

pub fn classify_intersection(
    line1: &LineSegment2,
    line2: &LineSegment2,
    intersection: &Point2<f32>,
) -> IntersectionKind {
    // Compute the unit direction vectors for both lines.
    let vec1 = (line1.end - line1.start).normalize();
    let vec2 = (line2.end - line2.start).normalize();

    let dot = vec1.dot(&vec2).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();

    let deviation_deg = (angle_rad - std::f32::consts::FRAC_PI_2).abs().to_degrees();

    if deviation_deg > 10.0 {
        return IntersectionKind::Unknown;
    }

    let d1 = (intersection - line1.start).norm();
    let d2 = (intersection - line1.end).norm();
    let d3 = (intersection - line2.start).norm();
    let d4 = (intersection - line2.end).norm();

    let near_line1 = d1 < 15.0 || d2 < 15.0;
    let near_line2 = d3 < 15.0 || d4 < 15.0;
    match (near_line1, near_line2) {
        (false, false) => IntersectionKind::X, // Intersection is central in both lines.
        (true, false) | (false, true) => IntersectionKind::T, // One line ends at the intersection.
        (true, true) => IntersectionKind::L,   // Both lines end at the intersection.
    }
}
