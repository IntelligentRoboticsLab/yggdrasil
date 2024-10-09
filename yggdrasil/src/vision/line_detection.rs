use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use crate::vision::camera::Image;

use crate::vision::scan_lines::RegionColor;

use super::line::LineSegment2;
use super::scan_lines::{ScanLine, ScanLines};

use bevy::prelude::*;
use heimdall::{CameraLocation, Top};
use nalgebra::Point2;
use tasks::conditions::task_finished;
use tasks::CommandsExt;

const MAX_VERTICAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 15.;

const MAX_HORIZONTAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 15.;

const MAX_ALLOWED_MISTAKES: u32 = 1;

const MIN_POINTS_PER_LINE: usize = 1;

const MINIMUM_LINE_SLOPE: f32 = 0.05;

const MAX_PIXEL_DISTANCE: usize = 1;

/// Module that detect lines from scan-lines.
///
/// This module provides the following resources to the application:
/// - [`TopLines`]
pub struct LineDetectionPlugin;

impl Plugin for LineDetectionPlugin {
    // fn initialize(self, app: App) -> Result<App> {
    //     // app.add_system(line_detection_system.after(super::scan_lines::scan_lines_system))
    //     app.add_system(line_detection_system.after(super::scan_lines::scan_lines_system))
    //         .add_task::<ComputeTask<Result<TopLineDetectionData>>>()?
    //         .add_startup_system(start_line_detection_task)?
    //         .init_resource::<TopLineDetectionData>()
    // }

    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<DetectedLines<Top>>();
        app.add_systems(
            Update,
            detect_lines2::<Top>.run_if(task_finished::<Image<Top>>),
        );
    }
}

/// Detected lines for the camera location `T`.
#[derive(Default, Resource)]
pub struct DetectedLines<T: CameraLocation> {
    line_points: Vec<(f32, f32)>,
    line_points_next: Vec<(f32, f32)>,
    pub lines: Vec<LineSegment2>,
    lines_points: Vec<LinePoints>,
    _marker: std::marker::PhantomData<T>,
}

// NOTE: This needs to be implemented manually because of the `PhantomData`
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for DetectedLines<T> {
    fn clone(&self) -> Self {
        DetectedLines {
            line_points: self.line_points.clone(),
            line_points_next: self.line_points_next.clone(),
            lines: self.lines.clone(),
            lines_points: self.lines_points.clone(),
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Default)]
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

fn is_white_horizontal<T: CameraLocation>(
    column: usize,
    row: usize,
    scan_lines: &ScanLines<T>,
) -> Option<bool> {
    is_white(column, row, scan_lines.horizontal())
}

fn is_white_vertical<T: CameraLocation>(
    column: usize,
    row: usize,
    scan_lines: &ScanLines<T>,
) -> Option<bool> {
    is_white(column, row, scan_lines.vertical())
}

fn is_white(column: usize, row: usize, scan_line: &ScanLine) -> Option<bool> {
    use std::cmp::Ordering;

    scan_line
        .classified_scan_line_regions()
        .binary_search_by(|classified_reagion| {
            match classified_reagion.scan_line_region().region() {
                super::scan_lines::Region::Vertical {
                    x,
                    y_start: _,
                    y_end: _,
                } if *x < column && *x < column + MAX_PIXEL_DISTANCE => Ordering::Less,
                super::scan_lines::Region::Vertical {
                    x,
                    y_start: _,
                    y_end: _,
                } if *x > column && *x > column - MAX_PIXEL_DISTANCE => Ordering::Greater,
                super::scan_lines::Region::Vertical {
                    x: _,
                    y_start,
                    y_end,
                } => {
                    if row < *y_start {
                        Ordering::Less
                    } else if row > *y_end {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                }
                super::scan_lines::Region::Horizontal {
                    y,
                    x_start: _,
                    x_end: _,
                } if *y < row && *y + MAX_PIXEL_DISTANCE < row => Ordering::Less,
                super::scan_lines::Region::Horizontal {
                    y,
                    x_start: _,
                    x_end: _,
                } if *y > row && *y - MAX_PIXEL_DISTANCE > row => Ordering::Greater,
                super::scan_lines::Region::Horizontal {
                    y: _,
                    x_start,
                    x_end,
                } => {
                    if column < *x_start {
                        Ordering::Less
                    } else if column > *x_end {
                        Ordering::Greater
                    } else {
                        Ordering::Equal
                    }
                }
            }
        })
        .ok()
        .map(|index| {
            matches!(
                scan_line.classified_scan_line_regions()[index].color(),
                RegionColor::WhiteOrBlack
            )
        })
}

// fn detect_top_lines(
//     line_detection_data: LineDetectionData,
//     line_spots: Vec<Point2<f32>>,
//     scan_lines: ScanLines,
// ) -> Result<TopLineDetectionData> {
//     Ok(TopLineDetectionData(
//         Some(?),
//         Some(scan_lines.image().clone()),
//     ))
// }

fn create_line_detection_data<T: CameraLocation>(
    line_detection_data: DetectedLines<T>,
    line_spots: Vec<Point2<f32>>,
    scan_lines: ScanLines<T>,
) -> DetectedLines<T> {
    let mut points = line_detection_data.line_points;
    // TODO: This clear should not be necessary.
    points.clear();
    points.extend(line_spots.iter().map(|point| (point.x, point.y)));
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
                    .unwrap_or((scan_lines.image().yuyv_image().height() as f32, 0f32));

            let start_column = line_points.start_column.min(point.0);
            let end_column = line_points.end_column.max(point.0);
            let start_row = line_points.start_row.min(point.1);
            let end_row = line_points.end_row.max(point.1);

            let mut allowed_mistakes = MAX_ALLOWED_MISTAKES;

            if end_row - start_row > end_column - start_column {
                for row in start_row as usize..end_row as usize {
                    let column = (row as f32 - intercept) / slope;
                    if column < 0f32 || column >= scan_lines.image().yuyv_image().width() as f32 {
                        continue;
                    }

                    if !is_white_horizontal(column as usize, row, &scan_lines).unwrap_or(true) {
                        if allowed_mistakes == 0 {
                            break;
                        }
                        allowed_mistakes -= 1;
                    }
                }
            } else {
                for column in start_column as usize..end_column as usize {
                    let row: f32 = slope * column as f32 + intercept;
                    if row < 0f32 || row >= scan_lines.image().yuyv_image().height() as f32 {
                        continue;
                    }

                    if !is_white_vertical(column, row as usize, &scan_lines).unwrap_or(true) {
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
        lines.push(line_points_to_line(line_points, scan_lines.image()));
    }

    points.clear();
    points_next.clear();

    DetectedLines {
        line_points: points,
        line_points_next: points_next,
        lines,
        lines_points,
        _marker: PhantomData,
    }
}

fn line_points_to_line<T: CameraLocation>(
    line_points: &LinePoints,
    image: &Image<T>,
) -> LineSegment2 {
    let mut start_column = line_points.start_column;
    let mut end_column = line_points.end_column;
    assert!(start_column <= end_column);

    let mut start_row = line_points.start_row;
    let mut end_row = line_points.end_row;
    assert!(start_row <= end_row);

    let (slope, intercept) = linreg::linear_regression_of::<f32, f32, f32>(&line_points.points)
        .unwrap_or((image.yuyv_image().height() as f32, 0.));

    if end_column - start_column < end_row - start_row {
        if !(-MINIMUM_LINE_SLOPE..MINIMUM_LINE_SLOPE).contains(&slope) {
            start_column = ((start_row - intercept) / slope)
                .min(image.yuyv_image().width() as f32 - 1.)
                .max(0.);
            end_column = ((end_row - intercept) / slope)
                .min(image.yuyv_image().width() as f32 - 1.)
                .max(0.);
        }
    } else if (-(1. / MINIMUM_LINE_SLOPE)..(1. / MINIMUM_LINE_SLOPE)).contains(&slope) {
        start_row = (start_column * slope + intercept)
            .min(image.yuyv_image().height() as f32 - 1.)
            .max(0.);
        end_row = (end_column * slope + intercept)
            .min(image.yuyv_image().height() as f32 - 1.)
            .max(0.);
    }

    // TODO: Remove these asserts.
    assert!(start_row >= 0.);
    assert!(end_row >= 0.);
    assert!(start_column >= 0.);
    assert!(end_column >= 0.);

    // TODO: Remove these asserts.
    assert!(start_row < image.yuyv_image().height() as f32);
    assert!(end_row < image.yuyv_image().height() as f32);
    assert!(start_column < image.yuyv_image().width() as f32);
    assert!(end_column < image.yuyv_image().width() as f32);

    LineSegment2::from_xy(start_column, start_row, end_column, end_row)
}

// TODO: Add this back
// fn draw_lines(
//     dbg: &DebugContext,
//     lines: &[LineSegment2],
//     image: &Image,
//     matrix: &CameraMatrix,
//     robot_pose: &RobotPose,
// ) -> Result<()> {
//     let all_lines = lines.iter().map(|line| line.into()).collect::<Vec<_>>();

//     dbg.log_lines2d_for_image("top_camera/image/lines", &all_lines, image, color::u8::RED)?;

//     let points_to_ground = all_lines
//         .iter()
//         .filter_map(|line| {
//             let (x1, y1) = line[0];
//             let (x2, y2) = line[1];

//             matrix
//                 .pixel_to_ground(point![x1, y1], 0.0)
//                 .ok()
//                 .and_then(|p1| {
//                     matrix
//                         .pixel_to_ground(point![x2, y2], 0.0)
//                         .ok()
//                         .map(|p2| [(p1[0], p1[1], p1[2]), (p2[0], p2[1], p2[2])])
//                 })
//         })
//         .collect::<Vec<_>>();

//     dbg.log_lines3d_for_image(
//         "top_camera/lines_3d",
//         &points_to_ground,
//         image,
//         color::u8::BLUE,
//     )?;
//     dbg.log_transformation("top_camera/lines_3d", &robot_pose.as_3d(), image)?;

//     Ok(())
// }

pub fn detect_lines2<T: CameraLocation>(
    mut commands: Commands,
    previous_lines: Option<Res<DetectedLines<T>>>,
    scan_lines: Res<ScanLines<T>>,
) {
    let line_spots = scan_lines
        .horizontal()
        .line_spots()
        .chain(scan_lines.vertical().line_spots())
        .collect();

    commands
        .prepare_task(tasks::TaskPool::AsyncCompute)
        .to_resource()
        .spawn({
            let previous_lines = previous_lines
                .map(|lines| lines.deref().clone())
                .unwrap_or_default();
            let scan_lines = scan_lines.deref().clone();

            async move {
                Some(create_line_detection_data(
                    previous_lines,
                    line_spots,
                    scan_lines,
                ))
            }
        });
}
