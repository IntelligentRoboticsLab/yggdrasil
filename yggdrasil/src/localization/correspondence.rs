use bevy::prelude::*;
use heimdall::CameraLocation;
use itertools::Itertools;
use nalgebra::{Point2, Vector2};

use crate::{
    core::{
        config::layout::{FieldLine, LayoutConfig, ParallelAxis},
        debug::{
            debug_system::{DebugAppExt, SystemToggle},
            DebugContext,
        },
    },
    nao::Cycle,
    vision::line_detection::{
        handle_line_task,
        line::{Line2, LineSegment2},
        DetectedLines,
    },
};

use super::RobotPose;

/// This plugin matches detected lines to their closest lines in the ideal field.
#[derive(Default)]
pub struct LineCorrespondencePlugin<T: CameraLocation>(std::marker::PhantomData<T>);

impl<T: CameraLocation> Plugin for LineCorrespondencePlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_logging::<T>)
            .add_systems(
                Update,
                (get_correspondences::<T>.after(handle_line_task::<T>),).chain(),
            )
            .add_named_debug_systems(
                Update,
                log_correspondences::<T>.after(get_correspondences::<T>),
                "Visualize line correspondences",
                SystemToggle::Enable,
            );
    }
}

/// The correspondence between a detected line and a field line.
#[derive(Debug, Clone)]
pub struct LineCorrespondence {
    pub pose: RobotPose,
    // The line segment detected in the field frame (should be robot frame)
    pub detected_line: LineSegment2,
    // The ideal field line that the detected line corresponds to
    pub field_line: FieldLine,
    // The line segment of the detected line projected onto the field line
    pub projected_line: LineSegment2,
    // The squared sum error (in meters) of the correspondence projection
    pub error: f32,
}

/// A collection of line correspondences.
#[derive(Debug, Deref, Component)]
pub struct LineCorrespondences(pub Vec<LineCorrespondence>);

/// Matches detected lines to their closest field lines.
pub fn get_correspondences<T: CameraLocation>(
    mut commands: Commands,
    layout: Res<LayoutConfig>,
    pose: Res<RobotPose>,
    lines: Query<(Entity, &DetectedLines), (With<T>, Added<DetectedLines>)>,
) {
    for (entity, lines) in lines.iter() {
        let mut correspondences = Vec::new();

        for segment in &lines.segments {
            // we want to find the best projection of the line onto a field line
            let mut best = None;

            let detected_line =
                LineSegment2::new(pose.inner * segment.start, pose.inner * segment.end);

            // closest angle to the positive x axis
            let abs_angle = {
                let line = detected_line.to_line();
                line.normal.y.atan2(line.normal.x)
            }
            .abs();

            for field_line in layout
                .field
                .field_lines()
                .into_iter()
                .filter(|line| match line {
                    FieldLine::Segment { axis, .. } => {
                        if abs_angle < std::f32::consts::FRAC_PI_2 {
                            matches!(axis, ParallelAxis::X)
                        } else {
                            matches!(axis, ParallelAxis::Y)
                        }
                    }
                    FieldLine::Circle(..) => true,
                })
            {
                // transform the detected line by the robot pose

                // project the line segment onto the current field line
                let (start, start_distance) = field_line.project_with_distance(detected_line.start);
                let (end, end_distance) = field_line.project_with_distance(detected_line.end);

                let error = start_distance.powi(2) + end_distance.powi(2);

                // only keep the correspondence that minimizes the error
                if best
                    .as_ref()
                    .is_some_and(|current: &LineCorrespondence| current.error < error)
                {
                    continue;
                }

                let projected_line = LineSegment2::new(start, end);

                best = Some(LineCorrespondence {
                    detected_line,
                    field_line,
                    projected_line,
                    error,
                    pose: pose.clone(),
                });
            }

            if let Some(correspondence) = best {
                correspondences.push(correspondence);
            }
        }

        commands
            .entity(entity)
            .insert(LineCorrespondences(correspondences));
    }
}

fn setup_logging<T: CameraLocation>(dbg: DebugContext) {
    let path = T::make_entity_path("localization/line_correspondences");

    dbg.log_with_cycle(
        path,
        Cycle::default(),
        &rerun::LineStrips3D::update_fields().with_colors([(0, 255, 255)]),
    );
}

fn log_correspondences<T: CameraLocation>(
    dbg: DebugContext,
    correspondences: Query<(&Cycle, &LineCorrespondences), (With<T>, Added<LineCorrespondences>)>,
) {
    for (cycle, correspondences) in correspondences.iter() {
        let path = T::make_entity_path("localization/line_correspondences");

        // projection lines from detected line to field line
        let lines = correspondences
            .0
            .iter()
            .flat_map(|c| {
                [
                    [
                        [c.detected_line.start.x, c.detected_line.start.y, 0.0],
                        [c.projected_line.start.x, c.projected_line.start.y, 0.0],
                    ],
                    [
                        [c.detected_line.end.x, c.detected_line.end.y, 0.0],
                        [c.projected_line.end.x, c.projected_line.end.y, 0.0],
                    ],
                ]
            })
            .collect_vec();

        dbg.log_with_cycle(
            path.as_str(),
            *cycle,
            &rerun::LineStrips3D::update_fields().with_strips(&lines),
        );
    }
}
