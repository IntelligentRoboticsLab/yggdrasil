use std::{collections::HashMap, ops::Deref};

// use nalgebra::Point2;

use nidhogg::types::{color, Rgb};

use crate::{
    // camera::Image,
    debug::DebugContext,
    prelude::*,
    vision::scan_lines::{scan_lines_system, PixelColor, ScanGrid, TopScanGrid},
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

fn find_black_segments(grid: &ScanGrid) -> HashMap<usize, Vec<Segment>> {
    let mut black_segments = HashMap::new();

    let vertical_scan_lines = grid.vertical();
    for (vertical_line_id, &column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
        let column = vertical_scan_lines.line(vertical_line_id);

        // reset every column
        let mut segments = Vec::new();
        let mut prev_is_black = false;

        for (row_id, pixel) in column.iter().enumerate() {
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
fn get_proposals(grid: &TopScanGrid, dbg: &DebugContext) -> Result<()> {
    let segments = find_black_segments(grid);

    let grouped_segments = group_segments(grid, segments);

    // for (i, group) in grouped_segments.clone().into_iter().enumerate() {
    //     print!("{i}: ({} [", group[0].column_id);
    //     for segment in group {
    //         print!("[{}, {}], ", segment.start, segment.end);
    //     }

    //     print!("]), ");
    // }

    let lines = grouped_segments
        .iter()
        .flat_map(|g| {
            g.iter().map(|s| {
                [
                    (s.column_id as f32, s.start as f32),
                    (s.column_id as f32, s.end as f32),
                ]
            })
        })
        .collect::<Vec<_>>();

    dbg.log_lines2d_for_image(
        "top_camera/image/ball_groups",
        &lines,
        grid.image().deref().clone(),
        color::u8::ORANGE,
    );

    // println!("{:?}\n\n\n\n", grouped_segments);

    Ok(())
}
