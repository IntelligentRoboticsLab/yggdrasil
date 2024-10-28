pub mod arrsac;
pub mod line;

use core::f32;
use std::collections::HashSet;

use arrsac::Arrsac;
use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top, YuyvImage};
use itertools::Itertools;
use kira::command;
use line::{LineCandidate, LineSegment2};
use nalgebra::{point, DVector, Matrix2, Point2, SymmetricEigen, Vector2};
use nidhogg::types::color;
use rand::seq::SliceRandom;
use rand::{Rng, RngCore};
use rerun::Color;
use tasks::CommandsExt;

use super::{camera::Image, scan_lines::ScanLines};
use crate::core::debug::RerunStream;
use crate::{core::debug::DebugContext, localization::RobotPose};

const ARRSAC_INLIER_THRESHOLD: f32 = 0.08;
const LINE_SEGMENT_MIN_POINTS: usize = 4;
const LINE_SEGMENT_MIN_LENGTH_SPLIT: f32 = 0.2;
const LINE_SEGMENT_MAX_DISTANCE: f32 = 8.0;
const MAX_LINE_GAP_DISTANCE: f32 = 0.2;
const WHITE_TEST_SAMPLES: usize = 10;
const WHITE_TEST_SAMPLE_DISTANCE: f32 = 0.10;
const WHITE_TEST_MERGE_RATIO: f32 = 0.75;
// rad
const WHITE_TEST_MAX_ANGLE: f32 = 0.15;
const LINE_SEGMENT_MIN_LENGTH_MERGE: f32 = 0.3;
const FINAL_WHITE_TEST_MERGE_RATIO: f32 = 0.25;

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

    // let mut all_groups = scan_lines.vertical().line_spot_groups();
    // all_groups.extend(scan_lines.horizontal().line_spot_groups());

    let spots = scan_lines
        .vertical()
        .line_spots()
        .chain(scan_lines.horizontal().line_spots())
        .collect::<Vec<_>>();

    // we need at least two points to fit a line
    if spots.len() < 2 {
        return None;
    }

    let projected_spots = spots
        .iter()
        .filter_map(|p| pixel_to_ground(&camera_matrix, *p))
        .collect::<Vec<_>>();

    let mut arrsac = Arrsac::new(ARRSAC_INLIER_THRESHOLD as f64, rng.clone());

    let mut unused_spots = spots
        .iter()
        .filter_map(|p| pixel_to_ground(&camera_matrix, *p))
        .collect::<Vec<_>>();

    let mut line_candidates = vec![];

    const MAX_ITERS: usize = 10;
    for _ in 0..MAX_ITERS {
        unused_spots.shuffle(&mut rng);
        let Some((line, mut inlier_idx)) = arrsac.fit(unused_spots.iter().copied()) else {
            // probably no more good lines!
            break;
        };

        inlier_idx.sort_unstable();
        inlier_idx.reverse();
        let inliers = inlier_idx
            .into_iter()
            .map(|i| unused_spots.remove(i))
            .collect::<Vec<_>>();

        let candidate = LineCandidate::new(line, inliers);

        // split the line into segments if neighboring points are further apart than 0.15m
        let (candidates, remainder) = candidate.split_at_gap(MAX_LINE_GAP_DISTANCE);
        unused_spots.extend(remainder);

        let candidates = candidates
            .into_iter()
            .filter_map(|c| {
                let has_enough_inliers = c.n_inliers() >= LINE_SEGMENT_MIN_POINTS;
                let is_close_enough =
                    nalgebra::distance(&c.segment.center(), &pose.world_position())
                        < LINE_SEGMENT_MAX_DISTANCE;
                let is_long_enough = c.segment.length() > LINE_SEGMENT_MIN_LENGTH_SPLIT;

                if has_enough_inliers && is_close_enough && is_long_enough {
                    Some(c)
                } else {
                    // put the spots back :)
                    unused_spots.extend(c.inliers.into_iter());
                    None
                }
            })
            .collect::<Vec<_>>();

        line_candidates.extend(candidates);

        // if we don't have enough points to fit any more lines
        if unused_spots.len() < 2 {
            break;
        }
    }

    /// DEBUG
    let mut allsamplesaaa = vec![];
    let mut allsamplesbbb = vec![];
    let mut allsamplesccc = vec![];
    /// DEBUG
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

            let mut samplesaaa = vec![];
            let mut samplesbbb = vec![];
            let mut samplesccc = vec![];

            // TODO: sample based on the length of the segment too not just a fixed sample count
            for sample in connected.sample_uniform(WHITE_TEST_SAMPLES) {
                let normal = connected.normal();

                let tester1 = sample + normal * WHITE_TEST_SAMPLE_DISTANCE;
                let tester2 = sample - normal * WHITE_TEST_SAMPLE_DISTANCE;

                // project the points back to the image
                let (point_pixel, tester1_pixel, tester2_pixel) = (
                    ground_to_pixel(&camera_matrix, sample).unwrap(),
                    ground_to_pixel(&camera_matrix, tester1).unwrap(),
                    ground_to_pixel(&camera_matrix, tester2).unwrap(),
                );

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

                samplesaaa.extend([point_pixel, tester1_pixel, tester2_pixel]);
                samplesbbb.extend([
                    rerun::Color::from_rgb(0, 0, 255),
                    if test1 {
                        rerun::Color::from_rgb(0, 255, 0)
                    } else {
                        rerun::Color::from_rgb(255, 0, 0)
                    },
                    if test2 {
                        rerun::Color::from_rgb(0, 255, 0)
                    } else {
                        rerun::Color::from_rgb(255, 0, 0)
                    },
                ]);
                samplesccc.extend([rerun::Radius::from(1.0); 3]);
            }

            allsamplesaaa.extend(samplesaaa);
            allsamplesbbb.extend(samplesbbb);
            allsamplesccc.extend(samplesccc);

            let ratio = tests.iter().filter(|&&t| t).count() as f32 / tests.len() as f32;

            if ratio > WHITE_TEST_MERGE_RATIO {
                let c = line_candidates.remove(i);
                line_candidates[j].merge(c);
                break;
            }
        }
    }

    line_candidates.retain(|c| {
        c.segment.length() > LINE_SEGMENT_MIN_LENGTH_MERGE
        // && c.segment.white_test(
        //     scan_lines.image(),
        //     &camera_matrix,
        //     WHITE_TEST_SAMPLES,
        //     WHITE_TEST_SAMPLE_DISTANCE,
        //     FINAL_WHITE_TEST_MERGE_RATIO,
        // )
    });

    dbg.log_with_cycle(
        T::make_entity_path("white_test"),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(allsamplesaaa.into_iter().map(|p| p.to_tuple()))
            .with_colors(allsamplesbbb)
            .with_radii(allsamplesccc),
    );

    let line_segments = line_candidates
        .iter()
        .cloned()
        .filter_map(|c| {
            let seg = c.segment;

            let Some(start) = ground_to_pixel(&camera_matrix, seg.start) else {
                return None;
            };

            let Some(end) = ground_to_pixel(&camera_matrix, seg.end) else {
                return None;
            };

            Some(LineSegment2::new(start, end))
        })
        .collect::<Vec<_>>();

    let colors = line_segments
        .iter()
        .map(|_| Color::from_rgb(rng.gen(), rng.gen(), rng.gen()));

    dbg.log_with_cycle(
        T::make_entity_path("line_segments_camera"),
        scan_lines.image().cycle(),
        &rerun::LineStrips2D::new(
            line_segments
                .iter()
                .map(|segment| segment.to_rerun_2d())
                .collect::<Vec<_>>(),
        )
        .with_colors(colors),
    );

    let line_segments = line_candidates
        .iter()
        .cloned()
        .map(|c| c.segment)
        .collect::<Vec<_>>();

    let line_segments_points = line_candidates
        .iter()
        .cloned()
        .map(|c| c.inliers)
        .collect::<Vec<_>>();

    let colors = line_segments
        .iter()
        .map(|_| Color::from_rgb(rng.gen(), rng.gen(), rng.gen()));

    dbg.log_with_cycle(
        T::make_entity_path("line_segments_cool"),
        scan_lines.image().cycle(),
        &rerun::LineStrips3D::new(
            line_segments
                .iter()
                .map(|segment| segment.to_rerun_3d())
                .collect::<Vec<_>>(),
        )
        .with_colors(colors),
    );

    let (colors, radii) = colors_and_radii_groups(&line_segments_points, 0.1, rng.clone());

    dbg.log_with_cycle(
        T::make_entity_path("line_segments_points_projected"),
        scan_lines.image().cycle(),
        &rerun::Points3D::new(
            line_segments_points
                .iter()
                .flat_map(|segment| segment.iter().map(|p| (p.x, p.y, 0.0)).collect::<Vec<_>>()),
        )
        .with_colors(colors)
        .with_radii(radii),
    );

    let line_segments_image = line_segments_points
        .iter()
        .map(|segment| {
            segment
                .iter()
                .flat_map(|p| ground_to_pixel(&camera_matrix, *p))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let (colors, radii) = colors_and_radii_groups(&line_segments_image, 2.0, rng.clone());

    dbg.log_with_cycle(
        T::make_entity_path(format!("line_segments_points")),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(
            line_segments_image
                .iter()
                .flat_map(|segment| segment.iter().map(|p| p.to_tuple()))
                .collect::<Vec<_>>(),
        )
        .with_colors(colors)
        .with_radii(radii),
    );

    // dbg.log_with_cycle(
    //     T::make_entity_path(format!("found_line")),
    //     scan_lines.image().cycle(),
    //     &rerun::LineStrips3D::new([[(x_min, y_min, 0.0), (x_max, y_max, 0.0)]])
    //         .with_colors([Color::from_rgb(0, 0, 255)]),
    // );

    let (colors, radii) = colors_and_radii(&spots, Color::from_rgb(0, 255, 255), 2.0);
    dbg.log_with_cycle(
        T::make_entity_path(format!("spots")),
        scan_lines.image().cycle(),
        &rerun::Points2D::new(spots.into_iter().map(|p| p.to_tuple()))
            .with_colors(colors)
            .with_radii(radii),
    );

    let (colors, radii) = colors_and_radii(&projected_spots, Color::from_rgb(0, 255, 255), 0.1);
    dbg.log_with_cycle(
        T::make_entity_path(format!("projected_spots")),
        scan_lines.image().cycle(),
        &rerun::Points3D::new(projected_spots.into_iter().map(|p| (p.x, p.y, 0.0)))
            .with_colors(colors)
            .with_radii(radii),
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
