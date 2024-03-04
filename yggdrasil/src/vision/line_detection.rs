use std::{mem, time::Instant};

use crate::camera::Image;
use crate::debug::DebugContext;
use crate::prelude::*;

use super::scan_lines::{BottomScanGrid, PixelColor, TopScanGrid};

pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(line_detection_system))
    }
}

struct Line {
    points: Vec<(f32, f32)>,
}

fn is_white(column: usize, row: usize, image: &Image) -> bool {
    let column = (column >> 1) << 1;
    assert_eq!(column % 2, 0);
    let offset = row * image.yuyv_image().width() * 2 + column * 2;

    let y1 = image.yuyv_image()[offset..offset + 4][0];
    let u = image.yuyv_image()[offset..offset + 4][1];
    let v = image.yuyv_image()[offset..offset + 4][3];

    let color = PixelColor::classify_yuv_pixel(y1, u, v);

    color == PixelColor::White
}

const MIN_ROW: usize = 166;
// const MIN_ROW: usize = 226;
// const MIN_ROW: usize = 170;

#[system]
fn line_detection_system(
    top_scan_grid: &mut TopScanGrid,
    bottom_scan_grid: &mut BottomScanGrid,
    dbg: &DebugContext,
) -> Result<()> {
    let start = Instant::now();
    let mut points = Vec::with_capacity(300);

    for horizontal_line_id in 0..top_scan_grid.horizontal().line_ids().len() {
        let row_id = *unsafe {
            top_scan_grid
                .horizontal()
                .line_ids()
                .get_unchecked(horizontal_line_id)
        };
        // TODO: Delete this if statement and use proper field boundary detection.
        if row_id < MIN_ROW {
            continue;
        }
        let row = top_scan_grid.horizontal().line(horizontal_line_id);

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

    for vertical_line_id in 0..top_scan_grid.vertical().line_ids().len() {
        let column_id = *unsafe {
            top_scan_grid
                .vertical()
                .line_ids()
                .get_unchecked(vertical_line_id)
        };
        let column = top_scan_grid.vertical().line(vertical_line_id);

        let mut start_opt = None;
        for row_id in MIN_ROW..column.len() {
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
    let mut points_next = Vec::<(f32, f32)>::new();

    let mut lines = Vec::<Line>::new();

    loop {
        if points.is_empty() {
            break;
        }

        let mut line = Line {
            points: vec![points[0]],
        };

        for point in points.iter().skip(1) {
            if (line.points.last().unwrap().0 - point.0).abs() > 20f32 {
                points_next.push(*point);
                continue;
            }
            let last_point = *line.points.last().unwrap();
            line.points.push(*point);

            // let Ok((slope, intercept)) =
            //     linreg::linear_regression_of::<f32, f32, f32>(&line.points)
            // else {
            //     // eprintln!("DETECTION HERE");
            //
            //     continue;
            // };
            let (slope, intercept) =
                match linreg::linear_regression_of::<f32, f32, f32>(&line.points) {
                    Ok((slope, intercept)) => (slope, intercept),
                    Err(err) => match err {
                        linreg::Error::TooSteep => (480f32, 0f32),
                        linreg::Error::Mean => todo!(),
                        linreg::Error::InputLenDif => todo!(),
                        linreg::Error::NoElements => todo!(),
                    },
                };

            let start_column = line.points.first().unwrap().0;
            let end_column = point.0;
            assert!(start_column <= end_column);

            let mut points_clone = line.points.clone();
            points_clone.sort_by(|(_col1, row1), (_col2, row2)| row1.partial_cmp(row2).unwrap());
            let start_row = points_clone.first().unwrap().1;
            let end_row = points_clone.last().unwrap().1;

            let mut allowed_mistakes = 4u32;

            if end_row - start_row > end_column - start_column {
                for row in start_row as usize..end_row as usize {
                    let column = (row as f32 - intercept) / slope;
                    if column < 0f32 || column >= 640f32 {
                        continue;
                    }

                    if !is_white(column as usize, row as usize, top_scan_grid.image()) {
                        if allowed_mistakes == 0 {
                            break;
                        }
                        allowed_mistakes -= 1;
                    }
                }
            } else {
                for column in start_column as usize..end_column as usize {
                    let mut row: f32 = slope * column as f32 + intercept;
                    if row < 0f32 || row >= 480f32 {
                        continue;
                    }

                    if !is_white(column, row as usize, top_scan_grid.image()) {
                        if allowed_mistakes == 0 {
                            break;
                        }
                        allowed_mistakes -= 1;
                    }
                }
            }
            if allowed_mistakes == 0 {
                line.points.pop().unwrap();
                points_next.push(*point);
            }
        }
        if line.points.len() > 4 {
            lines.push(line);
        } else {
            // TODO: Can this be unsorted now?
            line.points
                .iter()
                .skip(1)
                .for_each(|point| points_next.push(*point));
            // points_next.push(*line.points.first().unwrap());
            points_next.sort_by(|(col1, _row1), (col2, _row2)| col1.partial_cmp(col2).unwrap());
        }

        points.clear();
        mem::swap(&mut points, &mut points_next);
    }

    println!("elapsed: {:?}", start.elapsed());

    let mut all_line_points = Vec::<(f32, f32)>::new();

    for mut line in lines.iter_mut() {
        // eprintln!("len:{}", line.points.len());
        let start_column = line.points.first().unwrap().0;
        let end_column = line.points.last().unwrap().0;
        assert!(start_column <= end_column);

        let mut points_clone = line.points.clone();
        points_clone.sort_by(|(_col1, row1), (_col2, row2)| row1.partial_cmp(row2).unwrap());
        let start_row = points_clone.first().unwrap().1;
        let end_row = points_clone.last().unwrap().1;

        let Ok((slope, intercept)) = linreg::linear_regression_of::<f32, f32, f32>(&line.points)
        else {
            // eprintln!("DRAWING HERE");
            line.points
                .sort_by(|(_col1, row1), (_col2, row2)| row1.partial_cmp(row2).unwrap());

            let start_row = line.points.first().unwrap().1;
            let end_row = line.points.last().unwrap().1;
            let column = line.points.first().unwrap().1;

            for row in start_row as usize..end_row as usize {
                all_line_points.push((column as f32, row as f32));
            }

            continue;
        };

        if end_row - start_row > end_column - start_column {
            for row in start_row as usize..end_row as usize {
                let column = (row as f32 - intercept) / slope;
                if column < 0f32 || column >= 640f32 {
                    continue;
                }

                all_line_points.push((column as f32, row as f32));
            }
        } else {
            for column in start_column as usize..end_column as usize {
                let row = slope * column as f32 + intercept;
                if row < 0f32 || row >= 480f32 {
                    continue;
                }

                all_line_points.push((column as f32, row));
            }
        }

        // all_line_points.extend(&line.points[0..4]);
        // all_line_points.extend(&line.points);
    }

    dbg.log_points2d_for_image(
        "top_camera/image",
        &points_clone,
        top_scan_grid.image().clone(),
    )?;
    // dbg.log_points2d_for_image(
    //     "top_camera/image",
    //     &all_line_points,
    //     top_scan_grid.image().clone(),
    // )?;

    Ok(())
}

// #[system]
// fn line_detection_system(
//     top_scan_lines: &mut TopScanLines,
//     bottom_scan_lines: &mut BottomScanLines,
//     dbg: &DebugContext,
//     top_image: &TopImage,
// ) -> Result<()> {
//     // both horizontal and vertical
//     {
//         let mut points = Vec::with_capacity(30_000);
//
//         for horizontal_line_id in 21..top_scan_lines.row_ids().len() {
//             let row_id = *unsafe { top_scan_lines.row_ids().get_unchecked(horizontal_line_id) };
//             let row = top_scan_lines.horizontal_line(horizontal_line_id);
//
//             let mut start_opt = Option::<usize>::None;
//             for column_id in 0..row.len() {
//                 if row[column_id] == PixelColor::White {
//                     if start_opt.is_none() {
//                         start_opt = Some(column_id);
//                     }
//                 } else if let Some(start) = start_opt {
//                     if column_id - start < 30 {
//                         points.push((((column_id + start) / 2) as f32, row_id as f32));
//                     }
//                     start_opt = None;
//                 }
//             }
//         }
//
//         for vertical_line_id in 0..top_scan_lines.column_ids().len() {
//             let column_id = *unsafe { top_scan_lines.column_ids().get_unchecked(vertical_line_id) };
//             let column = top_scan_lines.vertical_line(vertical_line_id);
//
//             let mut start_opt = None;
//             for row_id in 166..column.len() {
//                 if column[row_id] == PixelColor::White {
//                     if start_opt.is_none() {
//                         start_opt = Some(row_id);
//                     }
//                 } else if let Some(start) = start_opt {
//                     if row_id - start < 30 {
//                         // if row_id - start > 1 && row_id - start < 10 {
//                         points.push((column_id as f32, ((row_id + start) / 2) as f32));
//                     }
//                     start_opt = None;
//                 }
//             }
//         }
//
//         let points_clone = points.clone();
//         points.sort_by(|(col1, _row1), (col2, _row2)| col1.partial_cmp(col2).unwrap());
//         let mut points_unused = Vec::<(f32, f32)>::new();
//
//         let mut lines = Vec::<Vec<(f32, f32)>>::new();
//         let mut ascending = true;
//
//         loop {
//             let mut line = Vec::new();
//             line.push(points[0]);
//
//             for point_id in 1..points.len() - 1 {
//                 let (col_id1, row_id1) = line.last().unwrap();
//                 let (col_id2, row_id2) = points[point_id];
//
//                 // if ascending {
//                 //     if row_id1 + 20f32 < row_id2 {
//                 //         points_unused.push((col_id2, row_id2));
//                 //         continue;
//                 //     }
//                 // } else {
//                 //     if row_id1 + 20f32 > row_id2 {
//                 //         points_unused.push((col_id2, row_id2));
//                 //         continue;
//                 //     }
//                 // }
//
//                 // if *row_id1 < row_id2 && col_id1 + 30f32 > col_id2 {
//                 // if *row_id1 > row_id2 {
//                 //     if col_id1 + 30f32 >= col_id2 {
//                 //         line.push((col_id2, row_id2));
//                 //     }
//                 if ascending {
//                     let row_diff = if line.len() > 2 {
//                         let (_, second_last_row) = line[line.len() - 2];
//                         let (_, last_row) = line[line.len() - 1];
//
//                         // eprintln!("diff: {}", (last_row - second_last_row) * 4.0);
//                         f32::min((last_row - second_last_row).abs() * 2.0, 30f32)
//                     } else {
//                         30f32
//                     };
//
//                     // if *row_id1 - row_diff < row_id2 && col_id1 + 30f32 > col_id2 {
//                     if *row_id1 - 20f32 < row_id2
//                         // if *row_id1 - 10f32 < row_id2
//                         && *row_id1 + row_diff> row_id2
//                         // && *row_id1 + 10f32> row_id2
//                         // && *row_id1 + row_diff > row_id2
//                         && col_id1 + 50f32 > col_id2
//                     {
//                         // if *row_id1 - 10.0 < row_id2 && col_id1 + 30f32 > col_id2 {
//                         line.push((col_id2, row_id2));
//                     } else {
//                         points_unused.push((col_id2, row_id2));
//                     }
//                 } else {
//                     // let diff = if line.len() > 2 {
//                     //     let (_, second_last_row) = line[line.len() - 2];
//                     //     let (_, last_row) = line[line.len() - 1];
//                     //
//                     //     (last_row - second_last_row) * 4.0
//                     // } else {
//                     //     30f32
//                     // };
//                     //
//                     // if *row_id1 + diff > row_id2 && col_id1 + 30f32 > col_id2 {
//                     //     // if *row_id1 + 10.0 > row_id2 && col_id1 + 30f32 > col_id2 {
//                     //     line.push((col_id2, row_id2));
//                     // } else {
//                     //     // points_unused.push((*col_id1, *row_id1));
//                     //     points_unused.push((col_id2, row_id2));
//                     // }
//                 }
//             }
//             // ascending = !ascending;
//
//             lines.push(line);
//
//             // eprintln!("points:        {}", points.len());
//             // eprintln!("unused_points: {}", points_unused.len());
//
//             std::mem::swap(&mut points, &mut points_unused);
//             // eprintln!("points:        {}", points.len());
//             // eprintln!("unused_points: {}", points_unused.len());
//
//             points_unused.clear();
//
//             // eprintln!("points:        {}", points.len());
//             // eprintln!("unused_points: {}\n\n", points_unused.len());
//
//             if points.len() == 0 {
//                 break;
//             }
//         }
//
//         // dbg.log_points2d_for_image("top_camera/image", &points, top_image.0.clone())?;
//         // dbg.log_points2d_for_image("top_camera/image", &points_clone, top_image.0.clone())?;
//         // dbg.log_points2d_for_image("top_camera/image", &line, top_image.0.clone())?;
//         // dbg.log_points2d_for_image("top_camera/image", &lines[0], top_image.0.clone())?;
//         // dbg.log_points2d_for_image("top_camera/image", &points_unused, top_image.0.clone())?;
//         dbg.log_points2d_for_image(
//             "top_camera/image",
//             // &lines.into_iter().flatten().collect::<Vec<(f32, f32)>>(),
//             &lines
//                 .into_iter()
//                 .filter(|vec| vec.len() > 4)
//                 .nth(2)
//                 .unwrap(),
//             top_image.0.clone(),
//         )?;
//     }
//
//     Ok(())
// }
