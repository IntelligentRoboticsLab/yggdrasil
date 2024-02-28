use std::ops::Not;
use std::process::exit;
use std::time::Instant;

use nalgebra::ComplexField;

use crate::prelude::*;
use crate::{camera::TopImage, debug::DebugContext};

use super::scan_lines::{BottomScanLines, PixelColor, ScanLines, TopScanLines};

pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(line_detection_system))
    }
}

#[system]
fn line_detection_system(
    top_scan_lines: &mut TopScanLines,
    bottom_scan_lines: &mut BottomScanLines,
    dbg: &DebugContext,
    top_image: &TopImage,
) -> Result<()> {
    // both horizontal and vertical
    {
        let mut points = Vec::with_capacity(30_000);

        for horizontal_line_id in 21..top_scan_lines.row_ids().len() {
            let row_id = *unsafe { top_scan_lines.row_ids().get_unchecked(horizontal_line_id) };
            let row = top_scan_lines.horizontal_line(horizontal_line_id);

            let mut start_opt = Option::<usize>::None;
            for column_id in 0..row.len() {
                if row[column_id] == PixelColor::White {
                    if start_opt.is_none() {
                        start_opt = Some(column_id);
                    }
                } else if let Some(start) = start_opt {
                    if column_id - start < 30 {
                        points.push((((column_id + start) / 2) as f32, row_id as f32));
                    }
                    start_opt = None;
                }
            }
        }

        for vertical_line_id in 0..top_scan_lines.column_ids().len() {
            let column_id = *unsafe { top_scan_lines.column_ids().get_unchecked(vertical_line_id) };
            let column = top_scan_lines.vertical_line(vertical_line_id);

            let mut start_opt = None;
            for row_id in 166..column.len() {
                if column[row_id] == PixelColor::White {
                    if start_opt.is_none() {
                        start_opt = Some(row_id);
                    }
                } else if let Some(start) = start_opt {
                    if row_id - start < 30 {
                        // if row_id - start > 1 && row_id - start < 10 {
                        points.push((column_id as f32, ((row_id + start) / 2) as f32));
                    }
                    start_opt = None;
                }
            }
        }

        let points_clone = points.clone();
        points.sort_by(|(col1, _row1), (col2, _row2)| col1.partial_cmp(col2).unwrap());
        let mut points_unused = Vec::<(f32, f32)>::new();

        let mut lines = Vec::<Vec<(f32, f32)>>::new();
        let mut ascending = true;

        loop {
            let mut line = Vec::new();
            line.push(points[0]);

            for point_id in 1..points.len() - 1 {
                let (col_id1, row_id1) = line.last().unwrap();
                let (col_id2, row_id2) = points[point_id];

                // if ascending {
                //     if row_id1 + 20f32 < row_id2 {
                //         points_unused.push((col_id2, row_id2));
                //         continue;
                //     }
                // } else {
                //     if row_id1 + 20f32 > row_id2 {
                //         points_unused.push((col_id2, row_id2));
                //         continue;
                //     }
                // }

                // if *row_id1 < row_id2 && col_id1 + 30f32 > col_id2 {
                // if *row_id1 > row_id2 {
                //     if col_id1 + 30f32 >= col_id2 {
                //         line.push((col_id2, row_id2));
                //     }
                if ascending {
                    let row_diff = if line.len() > 2 {
                        let (_, second_last_row) = line[line.len() - 2];
                        let (_, last_row) = line[line.len() - 1];

                        // eprintln!("diff: {}", (last_row - second_last_row) * 4.0);
                        f32::min((last_row - second_last_row).abs() * 2.0, 30f32)
                    } else {
                        30f32
                    };

                    // if *row_id1 - row_diff < row_id2 && col_id1 + 30f32 > col_id2 {
                    if *row_id1 - 20f32 < row_id2
                        // if *row_id1 - 10f32 < row_id2
                        && *row_id1 + row_diff> row_id2
                        // && *row_id1 + 10f32> row_id2
                        // && *row_id1 + row_diff > row_id2
                        && col_id1 + 50f32 > col_id2
                    {
                        // if *row_id1 - 10.0 < row_id2 && col_id1 + 30f32 > col_id2 {
                        line.push((col_id2, row_id2));
                    } else {
                        points_unused.push((col_id2, row_id2));
                    }
                } else {
                    // let diff = if line.len() > 2 {
                    //     let (_, second_last_row) = line[line.len() - 2];
                    //     let (_, last_row) = line[line.len() - 1];
                    //
                    //     (last_row - second_last_row) * 4.0
                    // } else {
                    //     30f32
                    // };
                    //
                    // if *row_id1 + diff > row_id2 && col_id1 + 30f32 > col_id2 {
                    //     // if *row_id1 + 10.0 > row_id2 && col_id1 + 30f32 > col_id2 {
                    //     line.push((col_id2, row_id2));
                    // } else {
                    //     // points_unused.push((*col_id1, *row_id1));
                    //     points_unused.push((col_id2, row_id2));
                    // }
                }
            }
            // ascending = !ascending;

            lines.push(line);

            // eprintln!("points:        {}", points.len());
            // eprintln!("unused_points: {}", points_unused.len());

            std::mem::swap(&mut points, &mut points_unused);
            // eprintln!("points:        {}", points.len());
            // eprintln!("unused_points: {}", points_unused.len());

            points_unused.clear();

            // eprintln!("points:        {}", points.len());
            // eprintln!("unused_points: {}\n\n", points_unused.len());

            if points.len() == 0 {
                break;
            }
        }

        // dbg.log_points2d_for_image("top_camera/image", &points, top_image.0.clone())?;
        // dbg.log_points2d_for_image("top_camera/image", &points_clone, top_image.0.clone())?;
        // dbg.log_points2d_for_image("top_camera/image", &line, top_image.0.clone())?;
        // dbg.log_points2d_for_image("top_camera/image", &lines[0], top_image.0.clone())?;
        // dbg.log_points2d_for_image("top_camera/image", &points_unused, top_image.0.clone())?;
        dbg.log_points2d_for_image(
            "top_camera/image",
            // &lines.into_iter().flatten().collect::<Vec<(f32, f32)>>(),
            &lines
                .into_iter()
                .filter(|vec| vec.len() > 4)
                .nth(2)
                .unwrap(),
            top_image.0.clone(),
        )?;
    }

    Ok(())
}
