use std::mem;

use crate::debug::DebugContext;
use crate::vision::camera::{matrix::CameraMatrices, Image};

use crate::prelude::*;

use super::line::LineSegment2;
use super::scan_lines::{PixelColor, ScanGrid, TopScanGrid};

use derive_more::Deref;
use heimdall::CameraMatrix;
use nalgebra::point;
use nidhogg::types::color;

const MAX_VERTICAL_LINE_WIDTH: usize = 50;

const MAX_HORIZONTAL_LINE_HEIGHT: usize = 30;

const MAX_VERTICAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 50.;

const MAX_HORIZONTAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 50.;

const MAX_ALLOWED_MISTAKES: u32 = 3;

const MIN_POINTS_PER_LINE: usize = 4;

const MINIMUM_LINE_SLOPE: f32 = 0.05;

/// Module that detect lines from scan-lines.
///
/// This module provides the following resources to the application:
/// - [`TopLines`]
pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(line_detection_system.after(super::scan_lines::scan_lines_system))
            .add_task::<ComputeTask<Result<TopLineDetectionData>>>()?
            .add_startup_system(start_line_detection_task)?
            .init_resource::<TopLineDetectionData>()
    }
}

#[derive(Default)]
struct LineDetectionData {
    line_points: Vec<(f32, f32)>,
    line_points_next: Vec<(f32, f32)>,
    lines: Vec<LineSegment2>,
    lines_points: Vec<LinePoints>,
}

#[derive(Default)]
pub struct TopLineDetectionData(Option<LineDetectionData>, Option<Image>);

#[derive(Deref)]
pub struct TopLines(#[deref] pub Vec<LineSegment2>, pub Image);

struct LinePoints {
    points: Vec<(f32, f32)>,
    start_column: f32,
    end_column: f32,
    start_row: f32,
    end_row: f32,
}

impl LinePoints {
    fn new(first_point: (f32, f32)) -> Self {
        LinePoints {
            points: vec![first_point],
            start_column: first_point.0,
            end_column: first_point.0,
            start_row: first_point.1,
            end_row: first_point.1,
        }
    }

    fn reuse(mut self, first_point: (f32, f32)) -> Self {
        self.points.clear();
        self.points.push(first_point);

        self.start_column = first_point.0;
        self.end_column = first_point.0;
        self.start_row = first_point.1;
        self.end_row = first_point.1;

        self
    }
}

fn is_white(column: usize, row: usize, image: &Image) -> bool {
    let column = (column >> 1) << 1;
    assert_eq!(column % 2, 0);
    let offset = row * image.yuyv_image().width() * 2 + column * 2;

    let y1 = unsafe { *image.yuyv_image().get_unchecked(offset) };
    let u = unsafe { *image.yuyv_image().get_unchecked(offset + 1) };
    let y2 = unsafe { *image.yuyv_image().get_unchecked(offset + 2) };
    let v = unsafe { *image.yuyv_image().get_unchecked(offset + 3) };

    PixelColor::yuyv_is_white(y1, u, y2, v)
}

fn extract_line_points(
    scan_grid: &ScanGrid,
    mut points: Vec<(f32, f32)>,
) -> Result<Vec<(f32, f32)>> {
    let boundary = scan_grid.boundary();

    for horizontal_line_id in 0..scan_grid.horizontal().line_ids().len() {
        let row_id = *unsafe {
            scan_grid
                .horizontal()
                .line_ids()
                .get_unchecked(horizontal_line_id)
        };
        let row = scan_grid.horizontal().line(horizontal_line_id);

        let mut start_opt = Option::<usize>::None;
        #[allow(clippy::needless_range_loop)]
        for column_id in 0..row.len() {
            if row_id < boundary.height_at_pixel(column_id as f32) as usize {
                continue;
            }

            if row[column_id] == PixelColor::White {
                if start_opt.is_none() {
                    start_opt = Some(column_id);
                }
            } else if let Some(start) = start_opt {
                if column_id - start < MAX_VERTICAL_LINE_WIDTH {
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
        for row_id in boundary.height_at_pixel(column_id as f32) as usize..column.len() {
            if column[row_id] == PixelColor::White {
                if start_opt.is_none() {
                    start_opt = Some(row_id);
                }
            } else if let Some(start) = start_opt {
                if row_id - start < MAX_HORIZONTAL_LINE_HEIGHT {
                    points.push((column_id as f32, ((row_id + start) / 2) as f32));
                }
                start_opt = None;
            }
        }
    }

    Ok(points)
}

fn detect_top_lines(
    line_detection_data: LineDetectionData,
    scan_grid: ScanGrid,
) -> Result<TopLineDetectionData> {
    let image = scan_grid.image().clone();
    Ok(TopLineDetectionData(
        Some(detect_lines(line_detection_data, scan_grid)?),
        Some(image.clone()),
    ))
}

fn detect_lines(
    line_detection_data: LineDetectionData,
    scan_grid: ScanGrid,
) -> Result<LineDetectionData> {
    let mut points = extract_line_points(&scan_grid, line_detection_data.line_points)?;
    points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    let mut points_next = line_detection_data.line_points_next;

    let mut lines_points_old = line_detection_data.lines_points;
    let mut lines_points = Vec::new();

    loop {
        if points.is_empty() {
            break;
        }

        let mut line_points = lines_points_old
            .pop()
            .map(|line_points| line_points.reuse(points[0]))
            .unwrap_or_else(|| LinePoints::new(points[0]));

        for point in points.iter().skip(1) {
            if (line_points.points.last().unwrap().0 - point.0).abs()
                > MAX_HORIZONTAL_DISTANCE_BETWEEN_LINE_POINTS
                || (line_points.points.last().unwrap().1 - point.1).abs()
                    > MAX_VERTICAL_DISTANCE_BETWEEN_LINE_POINTS
            {
                points_next.push(*point);
                continue;
            }
            line_points.points.push(*point);

            let (slope, intercept) =
                linreg::linear_regression_of::<f32, f32, f32>(&line_points.points)
                    .unwrap_or((scan_grid.height() as f32, 0f32));

            let start_column = line_points.start_column.min(point.0);
            let end_column = line_points.end_column.max(point.0);
            let start_row = line_points.start_row.min(point.1);
            let end_row = line_points.end_row.max(point.1);

            let mut allowed_mistakes = MAX_ALLOWED_MISTAKES;

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
                line_points.points.pop().unwrap();
                points_next.push(*point);
            } else {
                line_points.start_column = start_column;
                line_points.end_column = end_column;
                line_points.start_row = start_row;
                line_points.end_row = end_row;
            }
        }
        if line_points.points.len() >= MIN_POINTS_PER_LINE {
            lines_points.push(line_points);
        } else {
            points_next.extend(line_points.points.iter().skip(1));
            points_next.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
        }

        points.clear();
        mem::swap(&mut points, &mut points_next);
    }

    let mut lines = line_detection_data.lines;
    lines.clear();
    for line_points in lines_points.iter() {
        lines.push(line_points_to_line(line_points, &scan_grid));
    }

    points.clear();
    points_next.clear();

    Ok(LineDetectionData {
        line_points: points,
        line_points_next: points_next,
        lines,
        lines_points,
    })
}

fn line_points_to_line(line_points: &LinePoints, scan_grid: &ScanGrid) -> LineSegment2 {
    let mut start_column = line_points.start_column;
    let mut end_column = line_points.end_column;
    assert!(start_column <= end_column);

    let mut start_row = line_points.start_row;
    let mut end_row = line_points.end_row;
    assert!(start_row <= end_row);

    let (slope, intercept) = linreg::linear_regression_of::<f32, f32, f32>(&line_points.points)
        .unwrap_or((scan_grid.height() as f32, 0.));

    if end_column - start_column < end_row - start_row {
        if !(-MINIMUM_LINE_SLOPE..MINIMUM_LINE_SLOPE).contains(&slope) {
            start_column = ((start_row - intercept) / slope)
                .min(scan_grid.width() as f32 - 1.)
                .max(0.);
            end_column = ((end_row - intercept) / slope)
                .min(scan_grid.width() as f32 - 1.)
                .max(0.);
        }
    } else if (-(1. / MINIMUM_LINE_SLOPE)..(1. / MINIMUM_LINE_SLOPE)).contains(&slope) {
        start_row = (start_column * slope + intercept)
            .min(scan_grid.height() as f32 - 1.)
            .max(0.);
        end_row = (end_column * slope + intercept)
            .min(scan_grid.height() as f32 - 1.)
            .max(0.);
    }

    assert!(start_row >= 0.);
    assert!(end_row >= 0.);
    assert!(start_column >= 0.);
    assert!(end_column >= 0.);

    assert!(start_row < scan_grid.height() as f32);
    assert!(end_row < scan_grid.height() as f32);
    assert!(start_column < scan_grid.width() as f32);
    assert!(end_column < scan_grid.width() as f32);

    LineSegment2::from_xy(start_column, start_row, end_column, end_row)
}

fn draw_lines(
    dbg: &DebugContext,
    lines: &[LineSegment2],
    scan_grid: ScanGrid,
    matrix: &CameraMatrix,
) -> Result<()> {
    let all_lines = lines.iter().map(|line| line.into()).collect::<Vec<_>>();

    dbg.log_lines2d_for_image(
        "top_camera/image/lines",
        &all_lines,
        scan_grid.image(),
        color::u8::RED,
    )?;

    let points_to_ground = all_lines
        .iter()
        .filter_map(|line| {
            let (x1, y1) = line[0];
            let (x2, y2) = line[1];

            matrix
                .pixel_to_ground(point![x1, y1], 0.0)
                .ok()
                .and_then(|p1| {
                    matrix
                        .pixel_to_ground(point![x2, y2], 0.0)
                        .ok()
                        .map(|p2| [(p1[0], p1[1], p1[2]), (p2[0], p2[1], p2[2])])
                })
        })
        .collect::<Vec<_>>();

    dbg.log_lines3d_for_image(
        "top_camera/lines_3d",
        &points_to_ground,
        scan_grid.image(),
        color::u8::BLUE,
    )?;

    Ok(())
}

#[startup_system]
fn start_line_detection_task(
    storage: &mut Storage,
    top_scan_grid: &mut TopScanGrid,
    detect_top_lines_task: &mut ComputeTask<Result<TopLineDetectionData>>,
) -> Result<()> {
    storage.add_resource(Resource::new(TopLines(
        Vec::new(),
        top_scan_grid.image().clone(),
    )))?;

    let top_scan_grid = top_scan_grid.clone();
    detect_top_lines_task
        .try_spawn(move || detect_top_lines(Default::default(), top_scan_grid))
        .unwrap();

    Ok(())
}

#[system]
pub fn line_detection_system(
    top_scan_grid: &mut TopScanGrid,
    dbg: &DebugContext,
    detect_top_lines_task: &mut ComputeTask<Result<TopLineDetectionData>>,
    top_line_detection_data: &mut TopLineDetectionData,
    top_lines: &mut TopLines,
    camera_matrices: &CameraMatrices,
) -> Result<()> {
    if let Some(detect_lines_result) = detect_top_lines_task.poll() {
        *top_line_detection_data = detect_lines_result?;
        std::mem::swap(
            &mut top_lines.0,
            &mut top_line_detection_data.0.as_mut().unwrap().lines,
        );

        top_lines.1 = top_line_detection_data.1.clone().unwrap();
        draw_lines(
            dbg,
            &top_lines.0,
            top_scan_grid.clone(),
            &camera_matrices.top,
        )?;
    }

    if !detect_top_lines_task.active()
        && top_lines.1.timestamp() != top_scan_grid.image().timestamp()
    {
        let top_scan_grid = top_scan_grid.clone();
        let line_detection_data = top_line_detection_data.0.take().unwrap();
        detect_top_lines_task
            .try_spawn(move || detect_top_lines(line_detection_data, top_scan_grid))
            .unwrap();
    }

    Ok(())
}
