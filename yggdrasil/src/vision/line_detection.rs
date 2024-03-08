use std::mem;

use crate::camera::Image;
use crate::debug::DebugContext;
use crate::prelude::*;

use super::scan_lines::{PixelColor, ScanGrid, TopScanGrid};

pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(line_detection_system)
            .add_task::<ComputeTask<Result<Vec<Line>>>>()?
            .add_startup_system(start_line_detection_task)
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
    let y2 = image.yuyv_image()[offset..offset + 4][2];
    let v = image.yuyv_image()[offset..offset + 4][3];

    PixelColor::yuyv_is_white(y1, u, y2, v)
}

// TODO: Replace with proper field-boundary detection.
const MIN_ROW: usize = 240;

fn extract_line_points(scan_grid: &ScanGrid) -> Result<Vec<(f32, f32)>> {
    let mut points = Vec::with_capacity(300);

    for horizontal_line_id in 0..scan_grid.horizontal().line_ids().len() {
        let row_id = *unsafe {
            scan_grid
                .horizontal()
                .line_ids()
                .get_unchecked(horizontal_line_id)
        };
        if row_id < MIN_ROW {
            continue;
        }
        let row = scan_grid.horizontal().line(horizontal_line_id);

        let mut start_opt = Option::<usize>::None;
        #[allow(clippy::needless_range_loop)]
        for column_id in 0..row.len() {
            if row[column_id] == PixelColor::White {
                if start_opt.is_none() {
                    start_opt = Some(column_id);
                }
            } else if let Some(start) = start_opt {
                if column_id - start < 50 {
                    points.push((((column_id + start) / 2) as f32, row_id as f32));
                }
                start_opt = None;
            }
        }
    }

    for vertical_line_id in 0..scan_grid.vertical().line_ids().len() {
        let column_id = *unsafe {
            scan_grid
                .vertical()
                .line_ids()
                .get_unchecked(vertical_line_id)
        };
        let column = scan_grid.vertical().line(vertical_line_id);

        let mut start_opt = None;
        #[allow(clippy::needless_range_loop)]
        for row_id in MIN_ROW..column.len() {
            if column[row_id] == PixelColor::White {
                if start_opt.is_none() {
                    start_opt = Some(row_id);
                }
            } else if let Some(start) = start_opt {
                if row_id - start < 30 {
                    points.push((column_id as f32, ((row_id + start) / 2) as f32));
                }
                start_opt = None;
            }
        }
    }

    Ok(points)
}

fn detect_lines(scan_grid: ScanGrid) -> Result<Vec<Line>> {
    let mut points = extract_line_points(&scan_grid)?;
    points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    let mut points_next = Vec::<(f32, f32)>::with_capacity(300);

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
            line.points.push(*point);

            let (slope, intercept) = linreg::linear_regression_of::<f32, f32, f32>(&line.points)
                .unwrap_or((scan_grid.height() as f32, 0f32));
            let start_column = line
                .points
                .iter()
                .map(|(col, _)| col)
                .fold(f32::INFINITY, |col1, &col2| col1.min(col2));
            let end_column = line
                .points
                .iter()
                .map(|(col, _)| col)
                .fold(f32::NEG_INFINITY, |col1, &col2| col1.max(col2));
            assert!(start_column <= end_column);

            let start_row = line
                .points
                .iter()
                .map(|(_, row)| row)
                .fold(f32::INFINITY, |row1, &row2| row1.min(row2));
            let end_row = line
                .points
                .iter()
                .map(|(_, row)| row)
                .fold(f32::NEG_INFINITY, |row1, &row2| row1.max(row2));
            assert!(start_row <= end_row);

            let mut allowed_mistakes = 3u32;

            if end_row - start_row > end_column - start_column {
                for row in start_row as usize..end_row as usize {
                    let column = (row as f32 - intercept) / slope;
                    if column < 0f32 || column >= scan_grid.width() as f32 {
                        continue;
                    }

                    if !is_white(column as usize, row, scan_grid.image()) {
                        if allowed_mistakes == 0 {
                            break;
                        }
                        allowed_mistakes -= 1;
                    }
                }
            } else {
                for column in start_column as usize..end_column as usize {
                    let row: f32 = slope * column as f32 + intercept;
                    if row < 0f32 || row >= scan_grid.height() as f32 {
                        continue;
                    }

                    if !is_white(column, row as usize, scan_grid.image()) {
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
        if line.points.len() > 3 {
            lines.push(line);
        } else {
            points_next.extend(line.points.iter().skip(1));
            points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
        }

        points.clear();
        mem::swap(&mut points, &mut points_next);
    }

    Ok(lines)
}

fn draw_lines(dbg: &DebugContext, lines: &[Line], scan_grid: ScanGrid) -> Result<()> {
    let mut all_line_points = Vec::<(f32, f32)>::new();

    for line in lines {
        let (slope, intercept) = linreg::linear_regression_of::<f32, f32, f32>(&line.points)
            .unwrap_or((scan_grid.height() as f32, 0f32));
        let start_column = line
            .points
            .iter()
            .map(|(col, _)| col)
            .fold(f32::INFINITY, |col1, &col2| col1.min(col2));
        let end_column = line
            .points
            .iter()
            .map(|(col, _)| col)
            .fold(f32::NEG_INFINITY, |col1, &col2| col1.max(col2));
        assert!(start_column <= end_column);

        let start_row = line
            .points
            .iter()
            .map(|(_, row)| row)
            .fold(f32::INFINITY, |row1, &row2| row1.min(row2));
        let end_row = line
            .points
            .iter()
            .map(|(_, row)| row)
            .fold(f32::NEG_INFINITY, |row1, &row2| row1.max(row2));
        assert!(start_row <= end_row);

        if end_row - start_row > end_column - start_column {
            for row in start_row as usize..end_row as usize {
                let column = (row as f32 - intercept) / slope;
                if column < 0f32 || column >= scan_grid.width() as f32 {
                    continue;
                }

                all_line_points.push((column, row as f32));
            }
        } else {
            for column in start_column as usize..end_column as usize {
                let row = slope * column as f32 + intercept;
                if row < 0f32 || row >= scan_grid.height() as f32 {
                    continue;
                }

                all_line_points.push((column as f32, row));
            }
        }
    }

    dbg.log_points2d_for_image(
        "top_camera/image",
        &all_line_points,
        scan_grid.image().clone(),
    )?;

    Ok(())
}

#[startup_system]
fn start_line_detection_task(
    _storage: &mut Storage,
    top_scan_grid: &mut TopScanGrid,
    detect_lines_task: &mut ComputeTask<Result<Vec<Line>>>,
) -> Result<()> {
    let top_scan_grid = top_scan_grid.clone();
    detect_lines_task
        .try_spawn(move || detect_lines(top_scan_grid))
        .unwrap();

    Ok(())
}

#[system]
fn line_detection_system(
    top_scan_grid: &mut TopScanGrid,
    dbg: &DebugContext,
    detect_lines_task: &mut ComputeTask<Result<Vec<Line>>>,
) -> Result<()> {
    if let Some(detect_lines_result) = detect_lines_task.poll() {
        let lines = detect_lines_result?;
        draw_lines(dbg, &lines, top_scan_grid.clone())?;

        let top_scan_grid = top_scan_grid.clone();
        detect_lines_task
            .try_spawn(move || detect_lines(top_scan_grid))
            .unwrap();
    }

    Ok(())
}
