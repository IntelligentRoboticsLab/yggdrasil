use std::mem;

use crate::camera::Image;
use crate::debug::DebugContext;
use crate::prelude::*;

use super::scan_lines::{PixelColor, ScanGrid, TopScanGrid};

/// Module that detect lines from scan-lines.
///
/// This module provides the following resources to the application:
/// - <code>[Vec]<[Line]></code>
pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(line_detection_system.after(super::scan_lines::scan_lines_system))
            .add_task::<ComputeTask<Result<Vec<Line>>>>()?
            .add_startup_system(start_line_detection_task)?
            .init_resource::<Vec<Line>>()
    }
}

pub struct LinePoint {
    pub row: f32,
    pub column: f32,
}

pub struct Line(pub LinePoint, pub LinePoint);

struct LineBuilder {
    points: Vec<(f32, f32)>,
    start_column: f32,
    end_column: f32,
    start_row: f32,
    end_row: f32,
}

impl LineBuilder {
    fn new(first_point: (f32, f32)) -> Self {
        LineBuilder {
            points: vec![first_point],
            start_column: first_point.0,
            end_column: first_point.0,
            start_row: first_point.1,
            end_row: first_point.1,
        }
    }
}

fn is_white(column: usize, row: usize, image: &Image) -> bool {
    let column = (column >> 1) << 1;
    assert_eq!(column % 2, 0);
    let offset = row * image.yuyv_image().width() * 2 + column * 2;

    let y1 = image.yuyv_image()[offset];
    let u = image.yuyv_image()[offset + 1];
    let y2 = image.yuyv_image()[offset + 2];
    let v = image.yuyv_image()[offset + 3];

    PixelColor::yuyv_is_white(y1, u, y2, v)
}

// TODO: Replace with proper field-boundary detection.
const MIN_ROW: usize = 160;

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

    let mut line_builders = Vec::<LineBuilder>::new();

    loop {
        if points.is_empty() {
            break;
        }

        let mut line_builder = LineBuilder::new(points[0]);

        for point in points.iter().skip(1) {
            if (line_builder.points.last().unwrap().0 - point.0).abs() > 40f32
                || (line_builder.points.last().unwrap().1 - point.1).abs() > 40f32
            {
                points_next.push(*point);
                continue;
            }
            line_builder.points.push(*point);

            let (slope, intercept) =
                linreg::linear_regression_of::<f32, f32, f32>(&line_builder.points)
                    .unwrap_or((scan_grid.height() as f32, 0f32));

            let start_column = line_builder.start_column.min(point.0);
            let end_column = line_builder.end_column.max(point.0);
            let start_row = line_builder.start_row.min(point.1);
            let end_row = line_builder.end_row.max(point.1);

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
                line_builder.points.pop().unwrap();
                points_next.push(*point);
            } else {
                line_builder.start_column = start_column;
                line_builder.end_column = end_column;
                line_builder.start_row = start_row;
                line_builder.end_row = end_row;
            }
        }
        if line_builder.points.len() > 3 {
            line_builders.push(line_builder);
        } else {
            points_next.extend(line_builder.points.iter().skip(1));
            points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
        }

        points.clear();
        mem::swap(&mut points, &mut points_next);
    }

    Ok(line_builders
        .iter()
        .map(|line_builder| line_builder_to_line(line_builder, &scan_grid))
        .collect())
}

fn line_builder_to_line(line_builder: &LineBuilder, scan_grid: &ScanGrid) -> Line {
    let mut start_column = line_builder.start_column;
    let mut end_column = line_builder.end_column;
    assert!(start_column <= end_column);

    let mut start_row = line_builder.start_row;
    let mut end_row = line_builder.end_row;
    assert!(start_row <= end_row);

    let (slope, intercept) =
        linreg::linear_regression_of::<f32, f32, f32>(&line_builder.points).unwrap_or((scan_grid.height()., 0.));

    if end_column - start_column < end_row - start_row {
        if !(-0.05..0.05).contains(&slope) {
            start_column = ((start_row - intercept) / slope).min(scan_grid.width() - 1.).max(0.);
            end_column = ((end_row - intercept) / slope).min(scan_grid.width() - 1.).max(0.);
        }
    } else if (-20.0..20.).contains(&slope) {
        start_row = (start_column * slope + intercept).min(scan_grid.height() - 1.).max(0.);
        end_row = (end_column * slope + intercept).min(scan_grid.height() - 1.).max(0.);
    }

    assert!(start_row >= 0.);
    assert!(end_row >= 0.);
    assert!(start_column >= 0.);
    assert!(end_column >= 0.);

    assert!(start_row < scan_grid.height());
    assert!(end_row < scan_grid.height());
    assert!(start_column < scan_grid.width());
    assert!(end_column < scan_grid.width());

    Line(
        LinePoint {
            row: start_row,
            column: start_column,
        },
        LinePoint {
            row: end_row,
            column: end_column,
        },
    )
}

fn draw_lines(dbg: &DebugContext, lines: &[Line], scan_grid: ScanGrid) -> Result<()> {
    let all_lines = lines
        .iter()
        .map(
            |Line(
                LinePoint {
                    row: first_row,
                    column: first_column,
                },
                LinePoint {
                    row: second_row,
                    column: second_column,
                },
            )| [(*first_column, *first_row), (*second_column, *second_row)],
        )
        .collect::<Vec<_>>();

    dbg.log_lines2d_for_image("top_camera/lines", &all_lines, scan_grid.image().clone())?;

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
    lines: &mut Vec<Line>,
) -> Result<()> {
    if let Some(detect_lines_result) = detect_lines_task.poll() {
        *lines = detect_lines_result?;
        draw_lines(dbg, lines, top_scan_grid.clone())?;

        let points = extract_line_points(top_scan_grid)?;
        dbg.log_points2d_for_image(
            "top_camera/line_points",
            &points,
            top_scan_grid.image().clone(),
        )?;

        let top_scan_grid = top_scan_grid.clone();
        detect_lines_task
            .try_spawn(move || detect_lines(top_scan_grid))
            .unwrap();
    }

    Ok(())
}
