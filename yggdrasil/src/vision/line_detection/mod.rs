pub mod arrsac;
pub mod line;

use arrsac::Arrsac;
use bevy::prelude::*;
use bevy::tasks::{block_on, poll_once, AsyncComputeTaskPool, Task};
use heimdall::{CameraLocation, CameraMatrix, Top, YuyvImage};
use itertools::Itertools;
use line::{Line2, LineSegment2};
use nalgebra::{point, Point2};

use odal::Config;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{camera::Image, scan_lines::ScanLines};
use crate::core::config::layout::{FieldConfig, LayoutConfig};
use crate::core::debug::DebugContext;
use crate::localization::RobotPose;
use crate::nao::Cycle;
use crate::prelude::ConfigExt;

const MAX_LINES: usize = 10;

#[derive(Resource, Debug, Clone, Deserialize, Serialize, Reflect)]
#[serde(deny_unknown_fields)]
struct LineDetectionConfig {
    // margin outside of the field in which lines will still be considered
    pub field_margin: f32,
    // maximum number of iterations for ARRSAC
    pub max_iters: usize,
    // residual threshold for ARRSAC inliers
    pub arrsac_inlier_threshold: f32,
    // minimum number of points in a valid line segment
    pub line_segment_min_points: usize,
    // maximum distance of a valid line spot from the camera
    pub spot_max_distance: f32,
    // maximum distance between two inliers of a line
    pub max_line_gap_distance: f32,
    // number of samples for the white test
    pub white_test_samples: usize,
    // sampling distance for the white test
    pub white_test_sample_distance: f32,
    // ratio of white tests that need to pass for two lines to be merged
    pub white_test_merge_ratio: f32,
    // maximum angle in radians between two lines for them to be considered parallel
    pub white_test_max_angle: f32,
    // minimum length of a line segment after merging
    pub line_segment_min_length_post_merge: f32,
    // maximum length of a line segment after merging
    pub line_segment_max_length_post_merge: f32,
}

#[derive(Resource, Debug, Clone, Deserialize, Serialize, Reflect)]
#[serde(deny_unknown_fields)]
pub struct LineDetectionConfigs {
    pub top: LineDetectionConfig,
    pub bottom: LineDetectionConfig,
}

impl Config for LineDetectionConfigs {
    const PATH: &'static str = "line_detection.toml";
}

/// Plugin that adds systems to detect lines from scan-lines.
pub struct LineDetectionPlugin;

impl Plugin for LineDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<LineDetectionConfigs>().add_systems(
            Update,
            (
                detect_lines_system::<Top>.run_if(resource_exists_and_changed::<ScanLines<Top>>),
                handle_line_task,
                debug_lines::<Top>,
                debug_lines_inliers::<Top>,
                debug_lines_3d::<Top>,
                debug_rejected_lines::<Top>,
            ),
        );
    }
}

/// The lines that were detected in the image
#[derive(Component, Default)]
pub struct DetectedLines {
    /// The line equations of the lines that were detected
    pub lines: Vec<Line2>,
    /// The line segments that were detected
    pub segments: Vec<LineSegment2>,
    /// The inliers points of the lines that were detected
    pub inliers: Vec<Inliers>,
}

/// The line candidates that were rejected
#[derive(Component, Default, Deref, DerefMut)]
pub struct RejectedLines {
    /// Yes, this is deref polymorphism
    /// Yes, this might an anti-pattern
    /// No, I don't give a damn!
    #[deref]
    detected: DetectedLines,
    /// The reasons why each line was rejected
    pub rejections: Vec<Rejection>,
}

/// Reason why a line candidate was rejected
pub enum Rejection {
    TooShort,
    TooLong,
    NotEnoughSpots,
}

/// Inlier points of a line candidate
#[derive(Component, Debug, Deref, DerefMut)]
pub struct Inliers(Vec<Point2<f32>>);

impl Inliers {
    fn sort_by_x(&mut self) {
        self.0.sort_unstable_by(|a, b| a.x.total_cmp(&b.x));
    }

    /// Split the line candidate into multiple candidates, every time the gap between two neighboring inliers is too large
    ///
    /// Returns a vector of the separated line candidates
    fn split_at_gap(mut self, max_gap: f32) -> Vec<Self> {
        let mut candidates = vec![];

        while let Some(candidate) = self.split_at_gap_single(max_gap) {
            candidates.push(candidate);
        }
        candidates.push(self);

        // handle edge case where the first inlier is too far from the second
        candidates.retain(|c| c.len() >= 2);

        candidates
    }

    /// Split the line candidate into two candidates at the first point where the gap between two neighboring inliers is too large
    ///
    /// If no such point is found, leaves the candidate unchanged and returns `None`
    ///
    /// If such a point is found, mutates the current candidate and returns the new candidate that was split off
    fn split_at_gap_single(&mut self, max_gap: f32) -> Option<Self> {
        let split_index = self
            .iter()
            // (i, inlier)
            .enumerate()
            .rev()
            // ((i, inlier), (i-1, prev_inlier))
            .tuple_windows::<(_, _)>()
            .find_map(|((i, inlier), (_, prev_inlier))| {
                // find the first point where the gap between two neighboring inliers is too large
                if nalgebra::distance(inlier, prev_inlier) > max_gap {
                    Some(i)
                } else {
                    None
                }
            })?;

        let new_inliers = self.split_off(split_index);

        Some(Self(new_inliers))
    }
}

/// Candidate for a detected line
#[derive(Debug)]
struct LineCandidate {
    /// A line that was fitted on the inliers of the candidate
    line: Line2,
    /// Inlier points, sorted by x-coordinate
    inliers: Inliers,
    /// A line segment that connecting the first and last inlier
    segment: LineSegment2,
}

impl LineCandidate {
    /// Merge two line candidates into one
    fn merge(&mut self, other: LineCandidate) {
        // add the inliers and resort them
        self.inliers.0.extend(other.inliers.0);
        self.inliers.sort_by_x();

        // recompute the segment
        self.segment = LineSegment2::new(
            self.inliers.first().copied().unwrap(),
            self.inliers.last().copied().unwrap(),
        );
    }
}

#[derive(Component)]
struct LineTaskHandle(Task<(Vec<LineCandidate>, Vec<Option<Rejection>>)>);

fn detect_lines_system<T: CameraLocation>(
    mut commands: Commands,
    scan_lines: Res<ScanLines<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
    layout: Res<LayoutConfig>,
    pose: Res<RobotPose>,
    cfg: Res<LineDetectionConfigs>,
) {
    // TODO: Current tasks API is not flexible enough for this :)
    // Rewrite soon(tm) ?

    let cfg = match T::POSITION {
        heimdall::CameraPosition::Top => cfg.top.clone(),
        heimdall::CameraPosition::Bottom => cfg.bottom.clone(),
    };

    let cycle = scan_lines.image().cycle();
    let entity = commands.spawn(cycle).id();
    let pool = AsyncComputeTaskPool::get();

    let handle = pool.spawn({
        let scan_lines = scan_lines.clone();
        let camera_matrix = camera_matrix.clone();
        let field = layout.field.clone();
        let pose = pose.clone();

        async move { detect_lines(scan_lines, camera_matrix, field, pose, cfg) }
    });

    commands.entity(entity).insert(LineTaskHandle(handle));
}

fn handle_line_task(
    mut commands: Commands,
    mut lines: Query<Entity, Or<(With<DetectedLines>, With<RejectedLines>)>>,
    mut task_handles: Query<(Entity, &mut LineTaskHandle)>,
) {
    for (task_entity, mut task) in &mut task_handles {
        if let Some((candidates, rejections)) = block_on(poll_once(&mut task.0)) {
            // remove the old lines
            for entity in &mut lines {
                commands.entity(entity).despawn();
            }

            // and add the new lines
            let mut detected = DetectedLines::default();
            let mut rejected = RejectedLines::default();
            for (candidate, rejection) in candidates.into_iter().zip(rejections) {
                // TODO: this could be cleaner?
                if let Some(rejection) = rejection {
                    rejected.rejections.push(rejection);
                    rejected.lines.push(candidate.line);
                    rejected.segments.push(candidate.segment);
                    rejected.inliers.push(candidate.inliers);
                } else {
                    detected.lines.push(candidate.line);
                    detected.segments.push(candidate.segment);
                    detected.inliers.push(candidate.inliers);
                }
            }
            commands.entity(task_entity).insert(detected);
            commands.entity(task_entity).insert(rejected);
            commands.entity(task_entity).remove::<LineTaskHandle>();
        }
    }
}

fn detect_lines<T: CameraLocation>(
    scan_lines: ScanLines<T>,
    camera_matrix: CameraMatrix<T>,
    field: FieldConfig,
    pose: RobotPose,
    cfg: LineDetectionConfig,
) -> (Vec<LineCandidate>, Vec<Option<Rejection>>) {
    let mut projected_spots = scan_lines
        .line_spots()
        // project the points to the ground, in the field frame
        .flat_map(|p| camera_matrix.pixel_to_ground(p, 0.0).map(|p| p.xy()))
        .collect_vec();

    // filter out spots that are too far away
    projected_spots.retain(|p| p.coords.norm() < cfg.spot_max_distance);

    // filter out spots that are outside of the field (with some margin)
    projected_spots.retain(|p| {
        // apply the pose transformation to the spots first
        let position = pose.inner * p;
        field.in_field_with_margin(position, cfg.field_margin)
    });

    let mut candidates = vec![];

    let mut rng = rand::thread_rng();
    let mut arrsac = Arrsac::new(f64::from(cfg.arrsac_inlier_threshold), rng.clone());

    for _ in 0..cfg.max_iters {
        projected_spots.shuffle(&mut rng);
        let Some((line, inlier_idx)) = arrsac.fit(projected_spots.iter().copied()) else {
            // probably no more good lines!
            break;
        };

        // sort the inliers by x-coordinate
        let mut inliers = extract_indices(&mut projected_spots, inlier_idx);
        inliers.sort_unstable_by(|a, b| a.x.total_cmp(&b.x));

        let curr_candidates = Inliers(inliers)
            // split the line candidate into multiple candidates,
            // every time the gap between two neighboring inliers is too large
            .split_at_gap(cfg.max_line_gap_distance)
            .into_iter()
            // create a LineCandidate for each split
            .map(|inliers| {
                let segment = LineSegment2::new(
                    inliers.first().copied().unwrap(),
                    inliers.last().copied().unwrap(),
                );

                LineCandidate {
                    line,
                    inliers,
                    segment,
                }
            });

        candidates.extend(curr_candidates);
    }

    // check if we can merge two line candidates
    for i in (0..candidates.len()).rev() {
        for j in 0..i {
            let c1 = &candidates[i];
            let c2 = &candidates[j];

            // if the two lines are not parallel enough, skip
            if c1.line.normal.angle(&c2.line.normal) > cfg.white_test_max_angle {
                continue;
            }

            let center1 = c1.segment.center();
            let center2 = c2.segment.center();

            // the segment connecting the two centers
            let connected = LineSegment2::new(center1, center2);

            // if the segment connecting the centers is are not parallel enough, skip
            // stops the case where two lines are almost parallel, but they are far apart in the direction of their normal
            if connected.normal().angle(&c1.line.normal) > cfg.white_test_max_angle
                || connected.normal().angle(&c2.line.normal) > cfg.white_test_max_angle
            {
                continue;
            }

            // do a white test
            let mut tests = vec![];

            // TODO: sample based on the length of the segment too and not just a fixed sample count
            for sample in connected.sample_uniform(cfg.white_test_samples) {
                let normal = connected.normal();

                let tester1 = sample + normal * cfg.white_test_sample_distance;
                let tester2 = sample - normal * cfg.white_test_sample_distance;

                // project the points back to the image
                let Ok(sample_pixel) =
                    camera_matrix.ground_to_pixel(point![sample.x, sample.y, 0.0])
                else {
                    tests.extend([false, false]);
                    continue;
                };

                let image = scan_lines.image();

                let tester1_pixel = camera_matrix
                    .ground_to_pixel(point![tester1.x, tester1.y, 0.0])
                    .is_ok_and(|p| is_less_bright_and_more_saturated(sample_pixel, p, image));

                let tester2_pixel = camera_matrix
                    .ground_to_pixel(point![tester2.x, tester2.y, 0.0])
                    .is_ok_and(|p| is_less_bright_and_more_saturated(sample_pixel, p, image));

                tests.extend([tester1_pixel, tester2_pixel]);
            }

            // if the ratio of the white tests is high enough, merge the two candidates
            let ratio = tests.iter().filter(|&&t| t).count() as f32 / tests.len() as f32;

            if ratio > cfg.white_test_merge_ratio {
                let candidate = candidates.remove(i);
                candidates[j].merge(candidate);
                break;
            }
        }
    }

    // sort candidates by distance (closest first)
    candidates.sort_unstable_by(|a, b| {
        let distance_a = a.segment.center().coords.norm();
        let distance_b = b.segment.center().coords.norm();
        distance_a.total_cmp(&distance_b)
    });
    // and remove ones we don't need
    candidates.truncate(MAX_LINES);

    let rejections = candidates
        .iter()
        .map(|c| {
            let not_enough_spots = c.inliers.len() < cfg.line_segment_min_points;
            let is_too_short = c.segment.length() < cfg.line_segment_min_length_post_merge;
            let is_too_long = c.segment.length() > cfg.line_segment_max_length_post_merge;

            if not_enough_spots {
                Some(Rejection::NotEnoughSpots)
            } else if is_too_short {
                Some(Rejection::TooShort)
            } else if is_too_long {
                Some(Rejection::TooLong)
            } else {
                None
            }
        })
        .collect_vec();

    (candidates, rejections)
}

pub fn is_less_bright_and_more_saturated<T: CameraLocation>(
    p1: Point2<f32>,
    p2: Point2<f32>,
    image: &Image<T>,
) -> bool {
    #[inline]
    fn yhs_triple(p: Point2<f32>, image: &YuyvImage) -> Option<(f32, f32, f32)> {
        let (x, y) = (p.x as usize, p.y as usize);
        let pixel = image.pixel(x, y)?;
        Some(pixel.to_yhs2())
    }

    let (Some((y1, _h1, s1)), Some((y2, _h2, s2))) = (yhs_triple(p1, image), yhs_triple(p2, image))
    else {
        return false;
    };

    y1 < y2 && s1 > s2
}

/// Extracts the elements at the given indices from the vector and returns them in a new vector
///
/// Tries to be efficient by sorting the indices in descending order and removing the elements in reverse order
///
/// TODO: Can be with [`Vec::extract_if`](https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html#method.extract_if) when it gets stabilized
fn extract_indices<T>(vec: &mut Vec<T>, mut idx: Vec<usize>) -> Vec<T> {
    idx.sort_unstable();
    idx.reverse();
    idx.into_iter().map(|i| vec.remove(i)).collect_vec()
}

fn debug_lines<T: CameraLocation>(
    dbg: DebugContext,
    camera_matrix: Res<CameraMatrix<T>>,
    accepted: Query<(&Cycle, &DetectedLines), Added<DetectedLines>>,
) {
    for (cycle, lines) in accepted.iter() {
        dbg.log_with_cycle(
            T::make_entity_image_path("lines/detected"),
            *cycle,
            &rerun::LineStrips2D::new(
                lines
                    .segments
                    .iter()
                    .filter_map(|s| {
                        let (Ok(start), Ok(end)) = (
                            camera_matrix.ground_to_pixel(point![s.start.x, s.start.y, 0.0]),
                            camera_matrix.ground_to_pixel(point![s.end.x, s.end.y, 0.0]),
                        ) else {
                            return None;
                        };
                        Some(LineSegment2::new(start, end))
                    })
                    .map(<[(f32, f32); 2]>::from),
            )
            .with_colors(vec![(255, 255, 0); lines.segments.len()]),
        );
    }
}

fn debug_lines_3d<T: CameraLocation>(
    dbg: DebugContext,
    pose: Res<RobotPose>,
    accepted: Query<(&Cycle, &DetectedLines), Added<DetectedLines>>,
) {
    for (cycle, lines) in accepted.iter() {
        dbg.log_with_cycle(
            T::make_entity_path("lines/detected"),
            *cycle,
            &rerun::LineStrips3D::new(lines.segments.iter().map(|s| {
                let point = pose.inner * *s;
                [
                    (point.start.x, point.start.y, 0.0),
                    (point.end.x, point.end.y, 0.0),
                ]
            }))
            .with_colors(vec![(255, 255, 0); lines.segments.len()]),
        );
    }
}

fn debug_lines_inliers<T: CameraLocation>(
    dbg: DebugContext,
    camera_matrix: Res<CameraMatrix<T>>,
    accepted: Query<(&Cycle, &DetectedLines), Added<DetectedLines>>,
) {
    let mut rng = rand::thread_rng();
    for (cycle, lines) in accepted.iter() {
        let mut colors = vec![];
        let mut points = vec![];

        lines.inliers.iter().for_each(|inliers| {
            let c = (
                rng.gen_range(0..255),
                rng.gen_range(0..255),
                rng.gen_range(0..255),
            );

            let p = inliers
                .iter()
                .filter_map(|p| {
                    let Ok(point) = camera_matrix.ground_to_pixel(point![p.x, p.y, 0.0]) else {
                        return None;
                    };
                    Some(point)
                })
                .map(|p| (p.x, p.y))
                .collect_vec();

            colors.extend(vec![c; p.len()]);
            points.extend(p);
        });
        let radii = vec![2.0; points.len()];

        dbg.log_with_cycle(
            T::make_entity_path("lines/inliers"),
            *cycle,
            &rerun::Points2D::new(points)
                .with_colors(colors)
                .with_radii(radii),
        );
    }
}

fn debug_rejected_lines<T: CameraLocation>(
    dbg: DebugContext,
    camera_matrix: Res<CameraMatrix<T>>,
    rejected: Query<(&Cycle, &RejectedLines), Added<RejectedLines>>,
) {
    for (cycle, lines) in rejected.iter() {
        dbg.log_with_cycle(
            T::make_entity_image_path("lines/rejected"),
            *cycle,
            &rerun::LineStrips2D::new(
                lines
                    .segments
                    .iter()
                    .filter_map(|s| {
                        let (Ok(start), Ok(end)) = (
                            camera_matrix.ground_to_pixel(point![s.start.x, s.start.y, 0.0]),
                            camera_matrix.ground_to_pixel(point![s.end.x, s.end.y, 0.0]),
                        ) else {
                            return None;
                        };
                        Some(LineSegment2::new(start, end))
                    })
                    .map(<[(f32, f32); 2]>::from),
            )
            .with_colors(
                lines
                    .rejections
                    .iter()
                    .map(|r| match r {
                        Rejection::TooShort => (255, 0, 0),
                        Rejection::TooLong => (0, 255, 0),
                        Rejection::NotEnoughSpots => (0, 0, 255),
                    })
                    .collect_vec(),
            ),
        );
    }
}
