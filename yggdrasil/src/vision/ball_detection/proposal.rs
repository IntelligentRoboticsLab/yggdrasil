//! Module for finding possible ball locations from the top camera image

use std::{collections::HashMap, ops::Deref};

use nalgebra::Point2;
use nidhogg::types::color;
use serde::{Deserialize, Serialize};

use crate::{
    camera::{matrix::CameraMatrices, Image, TopImage},
    debug::DebugContext,
    prelude::*,
    vision::{
        field_boundary::FieldBoundary,
        scan_lines::{scan_lines_system, PixelColor, ScanGrid, TopScanGrid},
    },
};

/// Configurable values for getting ball proposals during the ball detection pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallProposalConfig {
    /// Maximum gap between black pixels to be considered a new segment
    pub white_ratio: f32,
    /// Maximum distance between two clusters to be considered the same cluster
    pub bounding_box_scale: f32,
    /// Height/width of the bounding box around the ball
    pub cluster_max_distance: f32,
    /// Ratio of white pixels in the local area around a point to be considered a ball
    pub black_segment_max_gap: usize,
    /// The proposals are often too low on the ball (possibly caused by shadows?),
    /// this causes the white ratio in the local area to too low.
    /// To fix this we offset the center of the proposal to be a bit higher
    pub center_offset: f32,
}

/// Module for finding possible ball locations in the top camera image
///
/// It adds the following resources to the app:
/// - [`BallProposals`]
pub struct BallProposalModule;

impl Module for BallProposalModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system_chain((get_proposals.after(scan_lines_system), log_proposals))
            .add_startup_system(init_ball_proposals)
    }
}

/// Points at which a ball may possibly be located
#[derive(Clone)]
pub struct BallProposals {
    pub image: Image,
    pub proposals: Vec<BallProposal>,
}

#[derive(Default, Clone)]
pub struct BallProposal {
    pub position: Point2<usize>,
    pub distance_to_ball: f32,
}

/// A segment of black pixels in a vertical scanline
#[derive(Clone, Hash, PartialEq, Eq)]
struct Segment {
    column_id: usize,
    start: usize,
    end: usize,
}

impl Segment {
    /// Check if the segment overlaps with another segment
    ///
    /// Overlapping is defined as the segments having at least one pixel in common
    fn overlaps(&self, other: &Segment) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/// Find all black segments in the vertical grid
fn find_black_segments(
    grid: &ScanGrid,
    boundary: &FieldBoundary,
    max_gap: usize,
) -> HashMap<usize, Vec<Segment>> {
    let mut black_segments = HashMap::new();

    // For every vertical scanline
    let vertical_scan_lines = grid.vertical();
    for (vertical_line_id, &column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
        let column = vertical_scan_lines.line(vertical_line_id);
        let boundary_top = boundary.height_at_pixel(column_id as f32);

        // Reset found segments in column
        let mut segments = Vec::new();
        let mut black_dist = max_gap;

        // For every pixel in the column that is below the boundary
        for (row_id, pixel) in column.iter().enumerate().skip(boundary_top as usize) {
            // Non-black pixel increases distance and continues
            if !matches!(pixel, PixelColor::Black) {
                black_dist += 1;
                continue;
            }

            // Add to a previous segment if it is close enough to the last black pixel
            if black_dist >= max_gap {
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

/// Cluster all segments of adjacent columns that overlap into a vec
fn cluster_segments(
    grid: &ScanGrid,
    black_segments: HashMap<usize, Vec<Segment>>,
) -> Vec<Vec<Segment>> {
    let column_ids = grid.vertical().line_ids();

    let mut curr_cluster = 0;
    let mut cluster_map = HashMap::new();

    for i in 0..column_ids.len() {
        let column_id = column_ids[i];

        for segment in black_segments[&column_id].iter() {
            let mut overlapping = false;

            // keep track if we already added this segment to a cluster
            // in case of C shaped segments
            let mut already_added = false;

            // check if the segment overlaps with the previous column
            if i != 0 {
                let prev_segments = &black_segments[&column_ids[i - 1]];
                for prev_segment in prev_segments {
                    if segment.overlaps(prev_segment) {
                        overlapping = true;

                        if already_added {
                            let cluster = cluster_map[segment];
                            cluster_map.insert(prev_segment, cluster);
                        } else {
                            let cluster = cluster_map[prev_segment];
                            cluster_map.insert(segment, cluster);
                            already_added = true;
                        }
                    }
                }
            }

            // create a new cluster if the segment does not overlap with the previous column
            if !overlapping {
                cluster_map.insert(segment, curr_cluster);
                curr_cluster += 1;
            }
        }
    }

    // turn into vector of clusters
    let mut clusters = Vec::new();

    for i in 0..curr_cluster {
        let cluster = cluster_map
            .iter()
            .filter(|(_, &cluster)| cluster == i)
            .map(|(&segment, _)| segment)
            .cloned()
            .collect::<Vec<_>>();

        if !cluster.is_empty() {
            clusters.push(cluster);
        }
    }

    clusters
}

/// Finds the center of a cluster of segments
fn cluster_centers(clusters: Vec<Vec<Segment>>) -> Vec<Point2<f32>> {
    clusters
        .iter()
        .map(|cluster| {
            let mut x = 0.0;
            let mut y = 0.0;

            for segment in cluster {
                x += segment.column_id as f32;
                y += (segment.start + segment.end) as f32 / 2.0;
            }

            Point2::new(
                (x / cluster.len() as f32).round(),
                (y / cluster.len() as f32).round(),
            )
        })
        .collect()
}

/// Find the ratio of ball colored (white/black) pixels in a local area around a point
fn local_white_ratio(range: usize, point: Point2<usize>, grid: &ScanGrid) -> f32 {
    let mut ball_colored = 0;
    let mut total = 0;

    let vertical_scan_lines = grid.vertical();
    let horizontal_scan_lines = grid.horizontal();

    // gap between horizontal pixels
    // TODO: This assumes the gap between vertical scan-lines is constant,
    // but that might not be the case in the future.
    let h_gap = grid.width() / vertical_scan_lines.line_ids().len();
    let v_gap = grid.height() / horizontal_scan_lines.line_ids().len();

    // TODO: this could be cleaner but I don't care #savage
    for pixel in &vertical_scan_lines.line(point.x / h_gap)
        [point.y.saturating_sub(range)..(point.y + range).min(grid.height())]
    {
        // count the ball colored pixels
        if matches!(pixel, PixelColor::White) {
            ball_colored += 1;
        }

        total += 1;
    }

    for pixel in &horizontal_scan_lines.line(point.y / v_gap)
        [point.y.saturating_sub(range)..(point.y + range).min(grid.width())]
    {
        // count the ball colored pixels
        if matches!(pixel, PixelColor::White) {
            ball_colored += 1;
        }

        total += 1;
    }

    ball_colored as f32 / total as f32
}

/// Merge cluster centers if they are close to each other
fn merge_clusters(mut centers: Vec<Point2<f32>>, max_distance: f32) -> Vec<Point2<usize>> {
    let mut clusters: Vec<Vec<Point2<f32>>> = Vec::new();

    'outer: while let Some(new_center) = centers.pop() {
        for cluster in &mut clusters {
            if cluster
                .iter()
                .any(|center| nalgebra::distance(center, &new_center) < max_distance)
            {
                cluster.push(new_center);
                continue 'outer;
            }
        }

        // new cluster
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

fn test_proposals(
    proposals: Vec<Point2<usize>>,
    grid: &ScanGrid,
    matrices: &CameraMatrices,
    config: &BallProposalConfig,
) -> Vec<BallProposal> {
    proposals
        .into_iter()
        .flat_map(|center| {
            // project point to ground to get distance
            // distance is used for the amount of surrounding pixels to sample
            let Ok(coord) = matrices.top.pixel_to_ground(center.cast(), 0.0) else {
                return None;
            };

            // area to look around the point (half the bounding box size)
            let scale = config.bounding_box_scale * 0.5;
            // get the distance from the robot to the point in order to scale the area we look around the point
            let magnitude = coord.coords.magnitude();

            Some((center, scale / magnitude, magnitude))
        })
        .filter(|&(center, range, magnitude)| {
            // TODO: find a better solution for this
            let offset = (config.center_offset / magnitude) as usize;
            let adjusted_center = Point2::new(center.x, center.y.saturating_sub(offset));

            local_white_ratio(range as usize, adjusted_center, grid) > config.white_ratio
        })
        .map(|(center, _, magnitude)| BallProposal {
            position: center,
            distance_to_ball: magnitude,
        })
        .collect::<Vec<_>>()
}

#[system]
pub(super) fn get_proposals(
    grid: &TopScanGrid,
    boundary: &FieldBoundary,
    matrices: &CameraMatrices,
    ball_proposals: &mut BallProposals,
    config: &BallProposalConfig,
) -> Result<()> {
    // TODO: find better way to do this
    // if the image has not changed, we don't need to recalculate the proposals
    if ball_proposals.image.timestamp() == grid.image().timestamp() {
        return Ok(());
    }

    // find black segments in each of the vertical scanlines
    let segments = find_black_segments(grid, boundary, config.black_segment_max_gap);
    // cluster the adjacent segments that overlap
    let clusters = cluster_segments(grid, segments);
    // find the center of the clusters
    let centers = cluster_centers(clusters);
    // merge the centers that are close to each other
    let potential_proposals = merge_clusters(centers, config.cluster_max_distance);
    // test the proposals for their white pixel ratio
    let proposals = test_proposals(potential_proposals, grid, matrices, config);

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
    config: &BallProposalConfig,
) -> Result<()> {
    let mut points = Vec::new();
    let mut sizes = Vec::new();
    for proposal in &ball_proposals.proposals {
        // project point to ground to get distance
        // distance is used for the amount of surrounding pixels to sample
        let Ok(coord) = matrices.top.pixel_to_ground(proposal.position.cast(), 0.0) else {
            continue;
        };

        let magnitude = coord.coords.magnitude();

        let size = config.bounding_box_scale / magnitude;

        points.push((proposal.position.x as f32, proposal.position.y as f32));
        sizes.push((size, size));
    }

    dbg.log_boxes_2d(
        "top_camera/image/ball_boxes",
        points.clone(),
        sizes,
        &ball_proposals.image,
        color::u8::SILVER,
    )?;

    dbg.log_points2d_for_image_with_radius(
        "top_camera/image/ball_spots",
        &points,
        ball_proposals.image.cycle(),
        color::u8::GREEN,
        4.0,
    )?;

    Ok(())
}

#[startup_system]
fn init_ball_proposals(storage: &mut Storage, image: &TopImage) -> Result<()> {
    let proposals = BallProposals {
        image: image.deref().clone(),
        proposals: Vec::new(),
    };

    storage.add_resource(Resource::new(proposals))
}
