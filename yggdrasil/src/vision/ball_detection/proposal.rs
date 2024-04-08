use std::collections::HashMap;

// use nalgebra::Point2;

use nidhogg::types::color;

use crate::{
    // camera::Image,
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

fn find_black_segments(grid: &ScanGrid, boundary: &FieldBoundary) -> HashMap<usize, Vec<Segment>> {
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

#[system]
fn get_proposals(grid: &TopScanGrid, boundary: &FieldBoundary, dbg: &DebugContext) -> Result<()> {
    let now = std::time::Instant::now();

    let segments = find_black_segments(grid, boundary);

    let grouped_segments = group_segments(grid, segments);

    println!("Took: {:?}", now.elapsed());

    for (i, group) in grouped_segments.clone().iter().enumerate() {
        print!("({i} [");
        for segment in group {
            print!(
                "[{}: {}, {}], ",
                segment.column_id, segment.start, segment.end
            );
        }

        print!("]), ");
    }

    println!("\n\n");

    println!("Len groups {:?}", grouped_segments.len());

    let lines = grouped_segments
        .iter()
        .flat_map(|g| {
            g.iter().flat_map(|s| {
                if s.start == s.end {
                    None
                } else {
                    Some([
                        (s.column_id as f32, s.start as f32),
                        (s.column_id as f32, s.end as f32),
                    ])
                }
            })
        })
        .collect::<Vec<_>>();

    let points = grouped_segments
        .iter()
        .flat_map(|g| {
            g.iter().flat_map(|s| {
                if s.start == s.end {
                    Some((s.column_id as f32, s.start as f32))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    dbg.log_lines2d_for_image(
        "top_camera/image/ball_groups_lines",
        &lines,
        grid.image().clone(),
        color::u8::TEAL,
    )?;

    dbg.log_points2d_for_image(
        "top_camera/image/ball_groups_points",
        &points,
        grid.image().clone(),
        color::u8::GREEN,
    )?;

    Ok(())
}
