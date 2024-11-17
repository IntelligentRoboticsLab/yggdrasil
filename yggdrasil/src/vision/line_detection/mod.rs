pub mod arrsac;
pub mod line;

use std::f32::consts::FRAC_PI_4;

use arrsac::Arrsac;
use bevy::prelude::*;
use bevy::tasks::{block_on, poll_once, AsyncComputeTaskPool, Task};
use heimdall::{CameraLocation, CameraMatrix, Top, YuyvImage};
use itertools::Itertools;
use line::{Line2, LineSegment2};
use nalgebra::{point, Point2};

use rand::seq::SliceRandom;

use super::{camera::Image, scan_lines::ScanLines};
use crate::core::debug::DebugContext;
use crate::localization::RobotPose;
use crate::nao::Cycle;

const MAX_ITERS: usize = 20;
const ARRSAC_INLIER_THRESHOLD: f32 = 0.025;
const LINE_SEGMENT_MIN_POINTS: usize = 5;
const SPOT_MAX_DISTANCE: f32 = 5.0;
const MAX_LINE_GAP_DISTANCE: f32 = 0.25;
const WHITE_TEST_SAMPLES: usize = 10;
const WHITE_TEST_SAMPLE_DISTANCE: f32 = 0.10;
const WHITE_TEST_MERGE_RATIO: f32 = 0.7;
// rad
const WHITE_TEST_MAX_ANGLE: f32 = FRAC_PI_4;
const LINE_SEGMENT_MIN_LENGTH_POST_MERGE: f32 = 0.3;
const LINE_SEGMENT_MAX_LENGTH_POST_MERGE: f32 = 5.0;

/// Plugin that adds systems to detect lines from scan-lines.
pub struct LineDetectionPlugin;

impl Plugin for LineDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                detect_lines_system::<Top>.run_if(resource_exists_and_changed::<ScanLines<Top>>),
                handle_line_task,
                debug_lines::<Top>,
                debug_rejected_lines::<Top>,
            ),
        );
    }
}

fn debug_lines<T: CameraLocation>(
    dbg: DebugContext,
    accepted: Query<(&Cycle, &DetectedLines), Added<DetectedLines>>,
) {
    for (cycle, lines) in accepted.iter() {
        dbg.log_with_cycle(
            T::make_entity_image_path("detected_lines"),
            *cycle,
            &rerun::LineStrips2D::new(lines.segments.iter().map(|s| <[(f32, f32); 2]>::from(*s)))
                .with_colors(vec![(180, 180, 180); lines.segments.len()]),
        );
    }
}

fn debug_rejected_lines<T: CameraLocation>(
    dbg: DebugContext,
    rejected: Query<(&Cycle, &RejectedLines), Added<RejectedLines>>,
) {
    for (cycle, lines) in rejected.iter() {
        dbg.log_with_cycle(
            T::make_entity_image_path("rejected_lines"),
            *cycle,
            &rerun::LineStrips2D::new(lines.segments.iter().map(|s| <[(f32, f32); 2]>::from(*s)))
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

pub enum Rejection {
    TooShort,
    TooLong,
    NotEnoughSpots,
}

#[derive(Component, Debug, Deref, DerefMut)]
pub struct Inliers(Vec<Point2<f32>>);

/// Candidate for a detected line
#[derive(Debug, Bundle)]
struct LineCandidate {
    /// A line that was fitted on the inliers of the candidate
    line: Line2,
    /// Inlier points, sorted by x-coordinate
    inliers: Inliers,
    /// A line segment that connecting the first and last inlier
    segment: LineSegment2,
}

impl LineCandidate {
    /// Merge the inliers of two line candidates
    fn merge(&mut self, other: LineCandidate) {
        self.inliers.0.extend(other.inliers.0);
        Self::sort_inliers(&mut self.inliers);
        self.segment = LineSegment2::new(
            self.inliers.first().copied().unwrap(),
            self.inliers.last().copied().unwrap(),
        );
    }

    fn sort_inliers(inliers: &mut [Point2<f32>]) {
        inliers.sort_unstable_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    }
}

impl Inliers {
    /// Split the line candidate into multiple candidates, every time the gap between two neighboring inliers is too large
    ///
    /// Returns a vector of the separated line candidates
    fn split_at_gap(mut self, max_gap: f32) -> Vec<Self> {
        let mut candidates = vec![];

        while let Some(candidate) = self.split_at_gap_single(max_gap) {
            candidates.push(candidate);
        }
        candidates.push(self);

        candidates
    }

    /// Split the line candidate into two candidates at the first point where the gap between two neighboring inliers is too large
    ///
    /// If no such point is found, leaves the candidate unchanged and returns `None`
    ///
    /// If such a point is found, mutates the current candidate and returns the new candidate that was split off
    fn split_at_gap_single(&mut self, max_gap: f32) -> Option<Self> {
        assert!(self.len() >= 2);

        let Some(split_index) = self
            .iter()
            // (i, inlier)
            .enumerate()
            .rev()
            // ((i, inlier), (i-1, prev_inlier))
            .tuples::<(_, _)>()
            .find_map(|((i, inlier), (_, prev_inlier))| {
                // find the first point where the gap between two neighboring inliers is too large
                if nalgebra::distance(inlier, prev_inlier) > max_gap {
                    Some(i)
                } else {
                    None
                }
            })
        // no split point found
        else {
            return None;
        };

        let new_inliers = self.split_off(split_index);

        Some(Self(new_inliers))
    }
}

#[derive(Component)]
struct LineTaskHandle(Task<(Vec<LineCandidate>, Vec<Option<Rejection>>)>);

fn detect_lines_system<T: CameraLocation>(
    mut commands: Commands,
    scan_lines: Res<ScanLines<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
    pose: Res<RobotPose>,
) {
    // TODO: Current tasks API is not flexible enough for this :)
    // Rewrite soon(tm) ?
    let cycle = scan_lines.image().cycle();
    let entity = commands.spawn(cycle).id();
    let pool = AsyncComputeTaskPool::get();

    let handle = pool.spawn({
        let scan_lines = scan_lines.clone();
        let camera_matrix = camera_matrix.clone();
        let pose = pose.clone();

        async move { detect_lines(scan_lines, camera_matrix, pose) }
    });

    commands.entity(entity).insert(LineTaskHandle(handle));
}

fn handle_line_task(
    mut commands: Commands,
    mut lines: Query<Entity, Or<(With<DetectedLines>, With<RejectedLines>)>>,
    mut task_handles: Query<(Entity, &mut LineTaskHandle)>,
) {
    for (task_entity, mut task) in task_handles.iter_mut() {
        if let Some((candidates, rejections)) = block_on(poll_once(&mut task.0)) {
            // remove the old lines
            for entity in lines.iter_mut() {
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
        }
    }
}

fn detect_lines<T: CameraLocation>(
    scan_lines: ScanLines<T>,
    camera_matrix: CameraMatrix<T>,
    pose: RobotPose,
) -> (Vec<LineCandidate>, Vec<Option<Rejection>>) {
    let mut projected_spots = scan_lines
        .line_spots()
        // project the points to the ground, in the field frame
        .flat_map(|p| camera_matrix.pixel_to_ground(p, 0.0).map(|p| p.xy()))
        .collect_vec();

    // filter out spots that are too far away
    projected_spots.retain(|p| p.coords.norm() < SPOT_MAX_DISTANCE);

    // TODO: filter out spots that are outside of the field (with some slack)
    // will need to apply the pose transformation to the spots first

    let mut candidates = vec![];

    let mut rng = rand::thread_rng();
    let mut arrsac = Arrsac::new(ARRSAC_INLIER_THRESHOLD as f64, rng.clone());

    for _ in 0..MAX_ITERS {
        projected_spots.shuffle(&mut rng);
        let Some((line, inlier_idx)) = arrsac.fit(projected_spots.iter().copied()) else {
            // probably no more good lines!
            break;
        };

        let curr_candidates = Inliers(extract_indices(&mut projected_spots, inlier_idx))
            // split the line candidate into multiple candidates,
            // every time the gap between two neighboring inliers is too large
            .split_at_gap(MAX_LINE_GAP_DISTANCE)
            .into_iter()
            // create a LineCandidate for each split
            .map(|inliers| {
                let segment = LineSegment2::new(
                    inliers.first().copied().unwrap(),
                    inliers.last().copied().unwrap(),
                );

                LineCandidate {
                    line,
                    segment,
                    inliers,
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
            if c1.line.normal.angle(&c2.line.normal) > WHITE_TEST_MAX_ANGLE {
                continue;
            }

            let center1 = c1.segment.center();
            let center2 = c2.segment.center();

            // the segment connecting the two centers
            let connected = LineSegment2::new(center1, center2);

            // if the segment connecting the centers is are not parallel enough, skip
            // stops the case where two lines are almost parallel, but they are far apart in the direction of their normal
            if connected.normal().angle(&c1.line.normal) > WHITE_TEST_MAX_ANGLE
                || connected.normal().angle(&c2.line.normal) > WHITE_TEST_MAX_ANGLE
            {
                continue;
            }

            // do a white test
            let mut tests = vec![];

            // TODO: sample based on the length of the segment too and not just a fixed sample count
            for sample in connected.sample_uniform(WHITE_TEST_SAMPLES) {
                let normal = connected.normal();

                let tester1 = sample + normal * WHITE_TEST_SAMPLE_DISTANCE;
                let tester2 = sample - normal * WHITE_TEST_SAMPLE_DISTANCE;

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
                    .map(|p| is_less_bright_and_more_saturated(sample_pixel, p, image))
                    .unwrap_or_default();

                let tester2_pixel = camera_matrix
                    .ground_to_pixel(point![tester2.x, tester2.y, 0.0])
                    .map(|p| is_less_bright_and_more_saturated(sample_pixel, p, image))
                    .unwrap_or_default();

                tests.extend([tester1_pixel, tester2_pixel]);
            }

            // if the ratio of the white tests is high enough, merge the two candidates
            let ratio = tests.iter().filter(|&&t| t).count() as f32 / tests.len() as f32;

            if ratio > WHITE_TEST_MERGE_RATIO {
                let candidate = candidates.remove(i);
                candidates[j].merge(candidate);
                break;
            }
        }
    }

    let rejections = candidates
        .iter()
        .map(|c| {
            let not_enough_spots = c.inliers.len() < LINE_SEGMENT_MIN_POINTS;
            let is_too_short = c.segment.length() < LINE_SEGMENT_MIN_LENGTH_POST_MERGE;
            let is_too_long = c.segment.length() > LINE_SEGMENT_MAX_LENGTH_POST_MERGE;

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
/// TODO: Can be with [`Vec::extract_if`] when it gets stabilized
/// https://doc.rust-lang.org/nightly/std/vec/struct.Vec.html#method.extract_if
fn extract_indices<T>(vec: &mut Vec<T>, mut idx: Vec<usize>) -> Vec<T> {
    idx.sort_unstable();
    idx.reverse();
    idx.into_iter().map(|i| vec.remove(i)).collect_vec()
}
