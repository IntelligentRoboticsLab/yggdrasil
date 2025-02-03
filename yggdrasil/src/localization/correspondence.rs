use bevy::prelude::*;
use heimdall::CameraLocation;
use itertools::Itertools;
use nalgebra::Point2;

use crate::{
    core::{
        config::layout::{FieldLine, LayoutConfig},
        debug::DebugContext,
    },
    nao::Cycle,
    vision::line_detection::{handle_line_task, line::LineSegment2, DetectedLines},
};

use super::RobotPose;

/// This plugin matches detected lines to their corresponding lines in field space.
#[derive(Default)]
pub struct LineCorrespondencePlugin<T: CameraLocation>(std::marker::PhantomData<T>);

impl<T: CameraLocation> Plugin for LineCorrespondencePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            get_correspondences::<T>.after(handle_line_task::<T>),
        );
    }
}

#[derive(Debug, Clone)]
pub struct Correspondence {
    pub detected_line: LineSegment2,
    pub field_line: FieldLine,

    pub error: f32,

    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

pub fn get_correspondences<T: CameraLocation>(
    dbg: DebugContext,
    layout: Res<LayoutConfig>,
    pose: Res<RobotPose>,
    lines: Query<(&Cycle, &DetectedLines), (With<T>, Added<DetectedLines>)>,
) {
    for (cycle, lines) in lines.iter() {
        let mut correspondences = Vec::new();
        // project segment end points onto all field lines
        for segment in &lines.segments {
            let mut closest = None;
            for field_line in layout.field.field_lines() {
                let FieldLine::Segment(segment_field) = field_line else {
                    continue;
                };

                let segment =
                    LineSegment2::new(pose.inner * segment.start, pose.inner * segment.end);

                // project segment end points onto the line
                let (start, start_distance) = segment_field.project_with_distance(segment.start);
                let (end, end_distance) = segment_field.project_with_distance(segment.end);

                let error = start_distance.powi(2) + end_distance.powi(2);

                match closest {
                    None => {
                        closest = Some(Correspondence {
                            detected_line: segment,
                            field_line,
                            error,
                            start,
                            end,
                        });
                    }
                    Some(ref current) => {
                        if error < current.error {
                            closest = Some(Correspondence {
                                detected_line: segment,
                                field_line,
                                error,
                                start,
                                end,
                            });
                        }
                    }
                };
            }

            if let Some(correspondence) = closest {
                correspondences.push(correspondence);
            }
        }

        let a = correspondences
            .iter()
            .flat_map(|c| {
                [
                    [
                        [c.detected_line.start.x, c.detected_line.start.y, 0.0],
                        [c.start.x, c.start.y, 0.0],
                    ],
                    [
                        [c.detected_line.end.x, c.detected_line.end.y, 0.0],
                        [c.end.x, c.end.y, 0.0],
                    ],
                ]
            })
            .collect_vec();

        dbg.log_with_cycle(
            "field_lines/correspondences",
            *cycle,
            &rerun::LineStrips3D::new(&a)
                .with_colors(vec![(255, 0, 255); a.len()])
                .with_radii(vec![0.02; a.len()]),
        );

        dbg.log_with_cycle(
            "field_lines/points",
            *cycle,
            &rerun::Points3D::new(correspondences.iter().flat_map(|c| {
                [
                    (c.detected_line.start.x, c.detected_line.start.y, 0.0),
                    (c.detected_line.end.x, c.detected_line.end.y, 0.0),
                ]
            }))
            .with_colors(vec![(255, 0, 0); correspondences.len() * 2])
            .with_radii(vec![0.1; correspondences.len() * 2]),
        );

        // dbg.log_with_cycle(
        //     "field_lines/line",
        //     *cycle,
        //     &rerun::LineStrips3D::new(
        //         flines
        //             .iter()
        //             .map(|l| [(l.start.x, l.start.y, 0.0), (l.end.x, l.end.y, 0.0)]),
        //     )
        //     .with_colors(vec![(255, 255, 0); flines.len()])
        //     .with_radii(vec![0.1; flines.len()]),
        // );
        // dbg.log(
        //     "field_lines/circle",
        //     &rerun::Ellipsoids3D::from_centers_and_radii(
        //         fcircles.iter().map(|(c, _r)| (c.x, c.y, 0.0)),
        //         fcircles.iter().flat_map(|(_c, r)| [*r, *r, 0.0]),
        //     ),
        // );
    }
}
