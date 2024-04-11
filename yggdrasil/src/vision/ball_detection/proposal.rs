use std::collections::HashMap;

use nalgebra::Point2;
use nidhogg::types::color;

use crate::{
    camera::{matrix::CameraMatrices, Image},
    debug::DebugContext,
    prelude::*,
    vision::{
        field_boundary::FieldBoundary,
        scan_lines::{scan_lines_system, PixelColor, ScanGrid, TopScanGrid},
    },
};

/// Maximum gap between black pixels to be considered a new segment
const BLACK_SEGMENT_MAX_GAP: usize = 3;
/// Maximum distance between two clusters to be considered the same cluster
const CLUSTER_MAX_DISTANCE: f32 = 64.0;
/// Height/width of the bounding box around the ball
const BOUNDING_BOX_SCALE: f32 = 64.0;
/// Ratio of white pixels in the local area around a point to be considered a ball
const WHITE_RATIO: f32 = 0.2;

pub struct BallProposalModule;

impl Module for BallProposalModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system_chain((get_proposals.after(scan_lines_system), log_proposals))
            .add_startup_system(init_ball_proposals)
    }
}

#[derive(Clone)]
pub struct BallProposals {
    pub image: Image,
    proposals: Vec<Point2<usize>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct Segment {
    column_id: usize,
    start: usize,
    end: usize,
}

impl Segment {
    fn overlaps(&self, other: &Segment) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/// Find all black segments in the vertical grid
fn find_black_segments(grid: &ScanGrid, boundary: &FieldBoundary) -> HashMap<usize, Vec<Segment>> {
    let mut black_segments = HashMap::new();

    // For every vertical scanline
    let vertical_scan_lines = grid.vertical();
    for (vertical_line_id, &column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
        let column = vertical_scan_lines.line(vertical_line_id);
        let boundary_top = boundary.height_at_pixel(column_id as f32);

        // Reset found segments in column
        let mut segments = Vec::new();
        let mut black_dist = BLACK_SEGMENT_MAX_GAP;

        // For every pixel in the column that is below the boundary
        for (row_id, pixel) in column.iter().enumerate().skip(boundary_top as usize) {
            // Non-black pixel increases distance and continues
            if !matches!(pixel, PixelColor::Black) {
                black_dist += 1;
                continue;
            }

            // Add to a previous segment
            if black_dist >= BLACK_SEGMENT_MAX_GAP {
                segments.push(Segment {
                    start: row_id,
                    end: row_id,
                    column_id,
                });
            // or make new segment
            } else {
                segments.last_mut().unwrap().end = row_id;
            }

            black_dist = 0;
        }

        // Insert the segments for this column
        black_segments.insert(column_id, segments);
    }

    black_segments
}

/// Group all segments that overlap if the columns are adjacent into a vec
fn group_segments(
    grid: &ScanGrid,
    black_segments: HashMap<usize, Vec<Segment>>,
) -> Vec<Vec<Segment>> {
    let column_ids = grid.vertical().line_ids();

    let mut curr_group = 0;
    let mut group_map = HashMap::new();

    for i in 0..column_ids.len() {
        let column_id = column_ids[i];

        for segment in black_segments[&column_id].iter() {
            let mut overlapping = false;

            // keep track if we already added this segment to a group
            // in case of C shaped segments
            let mut already_added = false;

            // check if the segment overlaps with the previous column
            if i != 0 {
                let prev_segments = &black_segments[&column_ids[i - 1]];
                for prev_segment in prev_segments {
                    if segment.overlaps(prev_segment) {
                        overlapping = true;

                        if already_added {
                            let group = group_map[segment];
                            group_map.insert(prev_segment, group);
                        } else {
                            let group = group_map[prev_segment];
                            group_map.insert(segment, group);
                            already_added = true;
                        }
                    }
                }
            }

            // create a new group if the segment does not overlap with the previous column
            if !overlapping {
                group_map.insert(segment, curr_group);
                curr_group += 1;
            }
        }
    }

    // turn into vector of groups
    let mut groups = Vec::new();

    for i in 0..curr_group {
        let group = group_map
            .iter()
            .filter(|(_, &group)| group == i)
            .map(|(&segment, _)| segment)
            .cloned()
            .collect::<Vec<_>>();

        if !group.is_empty() {
            groups.push(group);
        }
    }

    groups
}

/// Finds the center of a group of segments
fn group_centers(groups: Vec<Vec<Segment>>) -> Vec<Point2<f32>> {
    groups
        .iter()
        .map(|group| {
            let mut x = 0.0;
            let mut y = 0.0;

            for segment in group {
                x += segment.column_id as f32;
                y += (segment.start + segment.end) as f32 / 2.0;
            }

            Point2::new(
                (x / group.len() as f32).round(),
                (y / group.len() as f32).round(),
            )
        })
        .collect()
}

/// Find the ratio of ball colored (white/black) pixels in a local area around a point
fn local_white_ratio(range_h: usize, range_v: usize, point: Point2<usize>, grid: &ScanGrid) -> f32 {
    let mut ball_colored = 0;
    let mut total = 0;

    let vertical_scan_lines = grid.vertical();

    // gap between horizontal pixels
    let gap = grid.width() / vertical_scan_lines.line_ids().len();

    // TODO: this could be cleaner but I don't care #savage
    if range_h == 0 {
        for pixel in &vertical_scan_lines.line(point.x / gap)
            [point.y.saturating_sub(range_v)..(point.y + range_v).min(grid.height())]
        {
            // count the ball colored pixels
            if matches!(pixel, PixelColor::White) {
                ball_colored += 1;
            }

            total += 1;
        }
    } else {
        // look locally around the point
        for y in point.y.saturating_sub(range_v)..(point.y + range_v).min(grid.height()) {
            for x in point.x.saturating_sub(range_h)..(point.x + range_h).min(grid.width()) {
                let pixel = vertical_scan_lines.line(x / gap)[y];

                // count the ball colored pixels
                if matches!(pixel, PixelColor::White) {
                    ball_colored += 1;
                }

                total += 1;
            }
        }
    }

    ball_colored as f32 / total as f32
}

// merge cluster centers if they are close to each other
fn merge_groups(mut centers: Vec<Point2<f32>>) -> Vec<Point2<usize>> {
    let mut clusters: Vec<Vec<Point2<f32>>> = Vec::new();

    'outer: while let Some(new_center) = centers.pop() {
        for cluster in &mut clusters {
            if cluster
                .iter()
                .any(|center| nalgebra::distance(center, &new_center) < CLUSTER_MAX_DISTANCE)
            {
                cluster.push(new_center);
                continue 'outer;
            }
        }

        clusters.push(vec![new_center]);
    }

    // average the centers in the clusters
    clusters
        .into_iter()
        .map(|cluster| {
            let summed_position = cluster.iter().fold(Point2::default(), |acc, center| {
                Point2::new(acc.x + center.x, acc.y + center.y)
            });

            let mean_position = summed_position / cluster.len() as f32;

            mean_position.map(|x| x as usize)
        })
        .collect()
}

#[system]
fn get_proposals(
    grid: &TopScanGrid,
    boundary: &FieldBoundary,
    matrices: &CameraMatrices,
    ball_proposals: &mut BallProposals,
) -> Result<()> {
    // gap between horizontal pixels
    let gap = grid.width() / grid.vertical().line_ids().len();

    let segments = find_black_segments(grid, boundary);
    let groups = group_segments(grid, segments);
    let centers = group_centers(groups);

    let clusters = merge_groups(centers);

    let proposals = clusters
        .into_iter()
        .flat_map(|center| {
            // project point to ground to get distance
            // distance is used for the amount of surrounding pixels to sample
            let Ok(coord) = matrices.top.pixel_to_ground(center.cast(), 0.0) else {
                return None;
            };

            // area to look around the point (half the bounding box size)
            let scale = BOUNDING_BOX_SCALE * 0.5;

            let magnitude = coord.coords.magnitude();

            Some((center, scale / magnitude))
        })
        .filter(|&(center, magnitude)| {
            let hrange = ((magnitude / gap as f32) as usize).min(1);
            let vrange = magnitude as usize;

            local_white_ratio(hrange, vrange, center, grid) > WHITE_RATIO
        })
        .map(|(c, _)| c)
        .collect::<Vec<_>>();

    *ball_proposals = BallProposals {
        image: grid.image().clone(),
        proposals,
    };

    Ok(())
}

#[system]
fn log_proposals(
    dbg: &DebugContext,
    ball_proposals: &BallProposals,
    matrices: &CameraMatrices,
) -> Result<()> {
    let mut points = Vec::new();
    let mut sizes = Vec::new();
    for proposal in &ball_proposals.proposals {
        // project point to ground to get distance
        // distance is used for the amount of surrounding pixels to sample
        let Ok(coord) = matrices.top.pixel_to_ground(proposal.cast(), 0.0) else {
            continue;
        };

        // scale the ball to what the size it should be at this magnitude
        const BIG_SCALE: f32 = 64.0;

        let magnitude = coord.coords.magnitude();

        let size = BIG_SCALE / magnitude;

        points.push((proposal.x as f32, proposal.y as f32));
        sizes.push((size, size));
    }

    dbg.log_boxes_2d(
        "top_camera/image/ball_boxes",
        points.clone(),
        sizes.clone(),
        ball_proposals.image.clone(),
        color::u8::SILVER,
    )?;

    dbg.log_points2d_for_image_with_radius(
        "top_camera/image/ball_spots",
        &points,
        ball_proposals.image.clone(),
        color::u8::GREEN,
        4.0,
    )?;

    Ok(())
}

#[startup_system]
fn init_ball_proposals(storage: &mut Storage, grid: &ScanGrid) -> Result<()> {
    let proposals = BallProposals {
        image: grid.image().clone(),
        proposals: Vec::new(),
    };

    storage.add_resource(Resource::new(proposals))
}
