use std::collections::HashMap;

// use nalgebra::Point2;

use nalgebra::Point2;
use nidhogg::types::color;

use crate::{
    // camera::Image,
    camera::matrix::CameraMatrices,
    debug::DebugContext,
    prelude::*,
    vision::{
        field_boundary::FieldBoundary,
        scan_lines::{scan_lines_system, PixelColor, ScanGrid, TopScanGrid},
    },
};

pub struct BallProposalModule;

impl Module for BallProposalModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(get_proposals.after(scan_lines_system)))
    }
}

// struct BallProposal {
//     image: Image,
//     bbox: Bbox,
// }

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Segment {
    column_id: usize,
    start: usize,
    end: usize,
}

// struct Bbox {
//     top_left: Point2<usize>,
//     bottom_right: Point2<usize>,
// }

impl Segment {
    fn overlaps(&self, other: &Segment) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

fn find_black_segments_simple(
    grid: &ScanGrid,
    boundary: &FieldBoundary,
) -> HashMap<usize, Vec<Segment>> {
    let mut black_segments = HashMap::new();

    let vertical_scan_lines = grid.vertical();
    for (vertical_line_id, &column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
        let column = vertical_scan_lines.line(vertical_line_id);
        let boundary_top = boundary.height_at_pixel(column_id as f32);

        // reset every column
        let mut segments = Vec::new();
        let mut prev_is_black = false;

        for (row_id, pixel) in column.iter().enumerate().skip(boundary_top as usize) {
            // non-black pixel
            if !matches!(pixel, PixelColor::Black) {
                prev_is_black = false;
                continue;
            }

            // add to previous segment
            if !prev_is_black {
                segments.push(Segment {
                    start: row_id,
                    end: row_id,
                    column_id,
                });
            // make new segment
            } else {
                segments.last_mut().unwrap().end = row_id;
            }

            prev_is_black = true;
        }

        black_segments.insert(column_id, segments);
    }

    black_segments
}

fn find_black_segments(grid: &ScanGrid, boundary: &FieldBoundary) -> HashMap<usize, Vec<Segment>> {
    let max_gap = 5;

    let mut black_segments = HashMap::new();

    let vertical_scan_lines = grid.vertical();
    for (vertical_line_id, &column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
        let column = vertical_scan_lines.line(vertical_line_id);
        let boundary_top = boundary.height_at_pixel(column_id as f32);

        // reset every column
        let mut segments = Vec::new();
        let mut black_dist = max_gap;

        for (row_id, pixel) in column.iter().enumerate().skip(boundary_top as usize) {
            // non-black pixel
            if !matches!(pixel, PixelColor::Black) {
                black_dist += 1;
                continue;
            }

            // add to previous segment
            if black_dist >= max_gap {
                segments.push(Segment {
                    start: row_id,
                    end: row_id,
                    column_id,
                });
            // make new segment
            } else {
                segments.last_mut().unwrap().end = row_id;
            }

            black_dist = 0;
        }

        black_segments.insert(column_id, segments);
    }

    black_segments
}

fn group_segments(
    grid: &ScanGrid,
    black_segments: HashMap<usize, Vec<Segment>>,
) -> Vec<Vec<Segment>> {
    // Group all segments that overlap if the columns are adjacent into a vec
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

            if i != 0 {
                let prev_segments = &black_segments[&column_ids[i - 1]];
                for prev_segment in prev_segments {
                    if segment.overlaps(prev_segment) {
                        overlapping = true;

                        if already_added {
                            let group = group_map.get(segment).unwrap();
                            group_map.insert(prev_segment, *group);
                        } else {
                            let group = group_map.get(prev_segment).unwrap();
                            group_map.insert(segment, *group);
                            already_added = true;
                        }
                    }
                }
            }

            if !overlapping {
                group_map.insert(segment, curr_group);
                curr_group += 1;
            }
        }
    }

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

    if range_h == 0 {
        for pixel in &vertical_scan_lines.line(point.x / gap)
            [point.y.saturating_sub(range_v)..(point.y + range_v).min(grid.height())]
        {
            // count the ball colored pixels
            match pixel {
                PixelColor::White => ball_colored += 1,
                _ => (),
            };

            total += 1;
        }
    } else {
        // look locally around the point
        for y in point.y.saturating_sub(range_v)..(point.y + range_v).min(grid.height()) {
            for x in point.x.saturating_sub(range_h)..(point.x + range_h).min(grid.width()) {
                let pixel = vertical_scan_lines.line(x / gap)[y];

                // count the ball colored pixels
                match pixel {
                    PixelColor::White => ball_colored += 1,
                    _ => (),
                };

                // // count the ball colored pixels
                // match pixel {
                //     PixelColor::White | PixelColor::Black => ball_colored += 1,
                //     _ => (),
                // };

                total += 1;
            }
        }
    }

    ball_colored as f32 / total as f32
}

fn cluster_centers(mut centers: Vec<Point2<f32>>, max_dist: f32) -> Vec<Point2<usize>> {
    let mut clusters: Vec<Vec<Point2<f32>>> = Vec::new();

    'outer: while let Some(new_center) = centers.pop() {
        for cluster in &mut clusters {
            if cluster
                .iter()
                .find(|center| nalgebra::distance(center, &new_center) < max_dist)
                .is_some()
            {
                cluster.push(new_center);
                continue 'outer;
            }
        }

        clusters.push(vec![new_center]);
    }

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
    dbg: &DebugContext,
) -> Result<()> {
    let now = std::time::Instant::now();
    // gap between horizontal pixels
    let gap = grid.width() / grid.vertical().line_ids().len();

    let segments = find_black_segments(grid, boundary);

    // let max_groups = 100;
    let groups = group_segments(grid, segments);

    let centers = group_centers(groups);

    let cluster_dist = 64.0;
    let clusters = cluster_centers(centers.clone(), cluster_dist);

    let proposals = clusters
        // TODO: remove this clone
        .into_iter()
        .flat_map(|center| {
            // project point to ground to get distance
            // distance is used for the amount of surrounding pixels to sample
            let Ok(coord) = matrices.top.pixel_to_ground(center.cast(), 0.0) else {
                return None;
            };

            // scale the ball to what the size it should be at this magnitude
            const SCALE: f32 = 32.0;

            let magnitude = coord.coords.magnitude();

            Some((center, SCALE / magnitude))
        })
        .filter(|&(center, magnitude)| {
            let hrange = ((magnitude / gap as f32) as usize).min(1);
            let vrange = magnitude as usize;

            local_white_ratio(hrange, vrange, center, grid) > 0.2
        })
        .map(|(c, _m)| c)
        .collect::<Vec<_>>();

    let mut points2 = Vec::new();
    let mut sizes = Vec::new();
    for p in &proposals {
        // project point to ground to get distance
        // distance is used for the amount of surrounding pixels to sample
        let Ok(coord) = matrices.top.pixel_to_ground(p.cast(), 0.0) else {
            continue;
        };

        // scale the ball to what the size it should be at this magnitude
        const BIG_SCALE: f32 = 64.0;

        let magnitude = coord.coords.magnitude();

        let size = BIG_SCALE / magnitude;

        points2.push((p.x as f32, p.y as f32));
        sizes.push((size, size));
    }

    // println!("Took: {:?}\n", now.elapsed());

    let points = proposals
        .iter()
        .map(|p| (p.x as f32, p.y as f32))
        .collect::<Vec<_>>();

    dbg.log_points2d_for_image_with_radius(
        "top_camera/image/ball_segments",
        &centers
            .iter()
            .map(|p| (p.x as f32, p.y as f32))
            .collect::<Vec<_>>(),
        grid.image().clone(),
        color::u8::YELLOW,
        1.0,
    )?;

    dbg.log_boxes_2d(
        "top_camera/image/ball_boxes",
        points2.clone(),
        sizes.clone(),
        grid.image().clone(),
        color::u8::SILVER,
    )?;

    dbg.log_points2d_for_image_with_radius(
        "top_camera/image/ball_spots",
        &points,
        grid.image().clone(),
        color::u8::GREEN,
        4.0,
    )?;

    Ok(())
}
