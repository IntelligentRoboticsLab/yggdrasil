pub mod arrsac;
pub mod line;

use core::f32;
use std::f32::consts::FRAC_PI_4;

use arrsac::Arrsac;
use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top, YuyvImage};
use itertools::Itertools;
use line::{LineCandidate, LineSegment2};
use nalgebra::{point, DVector, Point2};
use rand::seq::SliceRandom;
use rand::{Rng, RngCore};
use rerun::Color;
use tasks::CommandsExt;

use super::{camera::Image, scan_lines::ScanLines};
use crate::core::debug::RerunStream;
use crate::{core::debug::DebugContext, localization::RobotPose};

const MAX_ITERS: usize = 20;
const ARRSAC_INLIER_THRESHOLD: f32 = 0.025;
const LINE_SEGMENT_MIN_POINTS: usize = 5;
const LINE_SEGMENT_MIN_LENGTH_SPLIT: f32 = 0.3;
const SPOT_MAX_DISTANCE: f32 = 5.0;
const MAX_LINE_GAP_DISTANCE: f32 = 0.25;
const WHITE_TEST_SAMPLES: usize = 10;
const WHITE_TEST_SAMPLE_DISTANCE: f32 = 0.10;
const WHITE_TEST_MERGE_RATIO: f32 = 0.75;
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
            detect_lines_system::<Top>.run_if(resource_exists_and_changed::<ScanLines<Top>>),
        );
    }
}

#[derive(Debug, Clone, Resource)]
pub struct DetectedLines {
    lines: Vec<LineCandidate>,
}

impl DetectedLines {
    fn new(lines: Vec<LineCandidate>) -> Self {
        Self { lines }
    }
}

fn detect_lines_system<T: CameraLocation>(
    mut commands: Commands,
    scan_lines: Res<ScanLines<T>>,
    camera_matrix: Res<CameraMatrix<T>>,
    pose: Res<RobotPose>,
    dbg: DebugContext,
) {
    commands
        .prepare_task(tasks::TaskPool::AsyncCompute)
        .to_resource()
        .spawn({
            let rerun_stream = dbg.stream().clone();
            let scan_lines = scan_lines.clone();
            let camera_matrix = camera_matrix.clone();
            let pose = pose.clone();

            async move { detect_lines(rerun_stream, scan_lines, camera_matrix, pose) }
        });
}

fn detect_lines<T: CameraLocation>(
    dbg: RerunStream,
    scan_lines: ScanLines<T>,
    camera_matrix: CameraMatrix<T>,
    pose: RobotPose,
) -> Option<DetectedLines> {
    let mut rng = rand::thread_rng();

    let mut projected_spots = scan_lines
        .line_spots()
        // project the points to the ground
        .flat_map(|p| camera_matrix.pixel_to_ground(p, 0.0))
        // apply robot pose
        .map(|p| (pose.as_3d() * p).xy())
        .collect_vec();

    projected_spots.retain(|p| nalgebra::distance(&p, &pose.world_position()) < SPOT_MAX_DISTANCE);

    // TODO: filter out spots that are outside of the field (with some slack)

    dbg.log_with_cycle(
        T::make_entity_path("line_spots"),
        scan_lines.image().cycle(),
        &rerun::Points3D::new(
            projected_spots
                .iter()
                .map(|p| (p.x, p.y, 0.0))
                .collect_vec(),
        )
        .with_colors(vec![Color::from_rgb(255, 255, 255); projected_spots.len()]),
    );

    let mut arrsac = Arrsac::new(ARRSAC_INLIER_THRESHOLD as f64, rng.clone());

    let mut line_candidates = vec![];

    for _ in 0..MAX_ITERS {
        projected_spots.shuffle(&mut rng);
        let Some((line, inlier_idx)) = arrsac.fit(projected_spots.iter().copied()) else {
            // probably no more good lines!
            break;
        };

        // remove the inliers from the
        let inliers = extract_indices(&mut projected_spots, inlier_idx);

        let candidates = LineCandidate::new(line, inliers).split_at_gap(MAX_LINE_GAP_DISTANCE);

        line_candidates.extend(candidates);
    }

    // check if we can merge two line candidates
    for i in (0..line_candidates.len()).rev() {
        for j in 0..i {
            let c1 = &line_candidates[i];
            let c2 = &line_candidates[j];

            if c1.line.normal.angle(&c2.line.normal) > WHITE_TEST_MAX_ANGLE {
                continue;
            }

            let center1 = c1.segment.center();
            let center2 = c2.segment.center();

            // the segment connecting the two centers
            let connected = LineSegment2::new(center1, center2);

            // do a white test
            let mut tests = vec![];

            let pose_inverse = pose.as_3d().inverse();

            // TODO: sample based on the length of the segment too not just a fixed sample count
            for sample in connected.sample_uniform(WHITE_TEST_SAMPLES) {
                let normal = connected.normal();

                let tester1 = sample + normal * WHITE_TEST_SAMPLE_DISTANCE;
                let tester2 = sample - normal * WHITE_TEST_SAMPLE_DISTANCE;

                let sample = (pose_inverse * sample.coords.push(0.0)).xy().into();
                let tester1 = (pose_inverse * tester1.coords.push(0.0)).xy().into();
                let tester2 = (pose_inverse * tester2.coords.push(0.0)).xy().into();

                // project the points back to the image
                if let (Some(point_pixel), Some(tester1_pixel), Some(tester2_pixel)) = (
                    ground_to_pixel(&camera_matrix, sample),
                    ground_to_pixel(&camera_matrix, tester1),
                    ground_to_pixel(&camera_matrix, tester2),
                ) {
                    let test1 = is_less_bright_and_more_saturated(
                        tester1_pixel,
                        point_pixel,
                        scan_lines.image(),
                    );
                    let test2 = is_less_bright_and_more_saturated(
                        tester2_pixel,
                        point_pixel,
                        scan_lines.image(),
                    );

                    tests.extend([test1, test2]);
                } else {
                    tests.extend([false, false]);
                }
            }

            let ratio = tests.iter().filter(|&&t| t).count() as f32 / tests.len() as f32;

            if ratio > WHITE_TEST_MERGE_RATIO {
                let candidate = line_candidates.remove(i);
                line_candidates[j].merge(candidate);
                break;
            }
        }
    }

    line_candidates.retain(|c| {
        let is_long_enough = c.segment.length() > LINE_SEGMENT_MIN_LENGTH_POST_MERGE;
        let is_short_enough = c.segment.length() < LINE_SEGMENT_MAX_LENGTH_POST_MERGE;
        let has_enough_spots = c.inliers.len() >= LINE_SEGMENT_MIN_POINTS;

        is_long_enough && is_short_enough && has_enough_spots
    });

    let camera_line_candidates = line_candidates
        .iter()
        .cloned()
        .filter_map(|c| {
            let LineSegment2 { start, end } = c.segment;

            // back from field to robot
            let inverse = pose.inner.inverse();

            let (Some(start), Some(end)) = (
                ground_to_pixel(&camera_matrix, inverse * start),
                ground_to_pixel(&camera_matrix, inverse * end),
            ) else {
                return None;
            };

            Some(LineSegment2::new(start, end))
        })
        .map(|segment| segment.to_rerun_2d())
        .collect_vec();

    let colors = vec![Color::from_rgb(255, 0, 0); camera_line_candidates.len()];
    dbg.log_with_cycle(
        T::make_entity_path("line_segments"),
        scan_lines.image().cycle(),
        &rerun::LineStrips2D::new(camera_line_candidates).with_colors(colors),
    );

    let colors = vec![Color::from_rgb(255, 0, 0); line_candidates.len()];
    dbg.log_with_cycle(
        T::make_entity_path("line_segments"),
        scan_lines.image().cycle(),
        &rerun::LineStrips3D::new(
            line_candidates
                .iter()
                .map(|c| c.segment.to_rerun_3d())
                .collect_vec(),
        )
        .with_colors(colors),
    );

    return Some(DetectedLines::new(line_candidates));
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

    let Some((y1, _h1, s1)) = yhs_triple(p1, image) else {
        return false;
    };

    let Some((y2, _h2, s2)) = yhs_triple(p2, image) else {
        return false;
    };

    y1 < y2 && s1 > s2
}

fn colors_and_radii_groups<R: RngCore>(
    data: &[Vec<Point2<f32>>],
    radius: f32,
    mut rng: R,
) -> (Vec<rerun::Color>, Vec<rerun::Radius>) {
    let mut colors = vec![];
    let mut radii = vec![];

    for group in data {
        // random color for each group
        let color = rerun::Color::from_rgb(rng.gen(), rng.gen(), rng.gen());
        let (c, r) = colors_and_radii(group, color, radius);

        colors.extend(c);
        radii.extend(r);
    }

    (colors, radii)
}

fn colors_and_radii(
    data: &[Point2<f32>],
    color: rerun::Color,
    radius: f32,
) -> (
    impl Iterator<Item = rerun::Color>,
    impl Iterator<Item = rerun::Radius>,
) {
    let radius = rerun::Radius::from(radius);

    (
        std::iter::repeat(color).take(data.len()),
        std::iter::repeat(radius).take(data.len()),
    )
}

/// Converts a vector of 2d points to two seperate nalgbra vectors of
/// coordinates
fn points_to_vectors(points: impl Iterator<Item = Point2<f32>>) -> (DVector<f32>, DVector<f32>) {
    let (x, y): (Vec<f32>, Vec<f32>) = points.map(|p| (p.x, p.y)).unzip();
    (DVector::<f32>::from_vec(x), DVector::<f32>::from_vec(y))
}

pub fn pixel_to_ground<T: CameraLocation>(
    camera_matrix: &CameraMatrix<T>,
    point: Point2<f32>,
) -> Option<Point2<f32>> {
    let ground = camera_matrix.pixel_to_ground(point, 0.0).ok()?;
    Some(ground.xy())
}

pub fn ground_to_pixel<T: CameraLocation>(
    camera_matrix: &CameraMatrix<T>,
    point: Point2<f32>,
) -> Option<Point2<f32>> {
    let camera = camera_matrix
        .ground_to_pixel(point![point.x, point.y, 0.0])
        .ok()?;
    Some(camera)
}

trait ToTuple {
    type Output;

    fn to_tuple(&self) -> Self::Output;
}

impl ToTuple for Point2<f32> {
    type Output = (f32, f32);

    fn to_tuple(&self) -> Self::Output {
        (self.x, self.y)
    }
}

fn extract_indices<T>(vec: &mut Vec<T>, mut idx: Vec<usize>) -> Vec<T> {
    idx.sort_unstable();
    idx.reverse();
    idx.into_iter().map(|i| vec.remove(i)).collect_vec()
}
