use std::marker::PhantomData;
use std::mem;
use std::ops::Deref;

use crate::core::debug::debug_system::DebugAppExt;
use crate::core::debug::DebugContext;
use crate::localization::RobotPose;
use crate::vision::camera::Image;

use crate::vision::scan_lines::RegionColor;

use super::camera::init_camera;
use super::line::{LineSegment2, LineSegment3};
use super::scan_lines::{ScanLine, ScanLines};

use bevy::prelude::*;
use heimdall::{CameraLocation, CameraMatrix, Top};
use nalgebra::Point2;
use rerun::external::glam::{Quat, Vec2, Vec3};
use tasks::conditions::task_finished;
use tasks::CommandsExt;

const MAX_VERTICAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 15.;

const MAX_HORIZONTAL_DISTANCE_BETWEEN_LINE_POINTS: f32 = 15.;

const MAX_ALLOWED_MISTAKES: u32 = 1;

const MIN_POINTS_PER_LINE: usize = 1;

const MINIMUM_LINE_SLOPE: f32 = 0.05;

const MAX_PIXEL_DISTANCE: usize = 1;

/// Plugin that adds systems to detect lines from scan-lines.
pub struct LineDetectionPlugin;

impl Plugin for LineDetectionPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Startup,
            (
                init_line_detection_data::<Top>,
                setup_line_visualization::<Top>,
            )
                .chain()
                .after(init_camera::<Top>),
        )
        .add_systems(
            Update,
            detect_lines::<Top>
                .run_if(resource_exists_and_changed::<ScanLines<Top>>)
                .run_if(task_finished::<DetectedLines<Top>>),
        )
        .add_named_debug_systems(
            PostUpdate,
            visualize_lines::<Top>.run_if(resource_exists_and_changed::<DetectedLines<Top>>),
            "Visualize lines",
        );
    }
}

/// Detected lines for the camera location `T`.
#[derive(Resource)]
pub struct DetectedLines<T: CameraLocation> {
    line_points: Vec<(f32, f32)>,
    line_points_next: Vec<(f32, f32)>,
    pub lines: Vec<LineSegment2>,
    pub projected_lines: Vec<LineSegment3>,
    lines_points: Vec<LinePoints>,
    image: Image<T>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: CameraLocation> DetectedLines<T> {
    /// Creates an empty [`DetectedLines`] instance with the given image.
    #[must_use]
    pub fn empty(image: Image<T>) -> Self {
        DetectedLines {
            line_points: Vec::new(),
            line_points_next: Vec::new(),
            lines: Vec::new(),
            projected_lines: Vec::new(),
            lines_points: Vec::new(),
            image,
            _marker: PhantomData,
        }
    }
}

// NOTE: This needs to be implemented manually because of the `PhantomData`
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for DetectedLines<T> {
    fn clone(&self) -> Self {
        DetectedLines {
            line_points: self.line_points.clone(),
            line_points_next: self.line_points_next.clone(),
            lines: self.lines.clone(),
            projected_lines: self.projected_lines.clone(),
            lines_points: self.lines_points.clone(),
            image: self.image.clone(),
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

fn init_line_detection_data<T: CameraLocation>(mut commands: Commands, image: Res<Image<T>>) {
    commands.insert_resource(DetectedLines::empty(image.clone()));
}

fn create_line_detection_data<T: CameraLocation>(
    line_detection_data: DetectedLines<T>,
    line_spots: Vec<Point2<f32>>,
    scan_lines: ScanLines<T>,
    matrix: &CameraMatrix<T>,
) -> DetectedLines<T> {
    let mut points = line_detection_data.line_points;
    // TODO: This clear should not be necessary.
    points.clear();
    points.extend(line_spots.iter().map(|point| (point.x, point.y)));
    points.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
    let mut points_next = line_detection_data.line_points_next;

    let mut lines_points_old = line_detection_data.lines_points;
    let mut current_line_points = Vec::new();

    loop {
        if points.is_empty() {
            break;
        }

        let mut line_points = lines_points_old.pop().map_or_else(
            || LinePoints::new(points[0]),
            |line_points| line_points.reuse(points[0]),
        );

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
            current_line_points.push(line_points);
        } else {
            points_next.extend(line_points.points.iter().skip(1));
            points_next.sort_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap());
        }

        points.clear();
        mem::swap(&mut points, &mut points_next);
    }

    let mut lines = line_detection_data.lines;
    lines.clear();
    for line_points in &current_line_points {
        lines.push(line_points_to_line(line_points, scan_lines.image()));
    }

    points.clear();
    points_next.clear();

    let projected_lines = lines
        .iter()
        .filter_map(|segment| segment.project_to_3d(matrix).ok())
        .collect();

    DetectedLines {
        line_points: points,
        line_points_next: points_next,
        lines,
        projected_lines,
        lines_points: current_line_points,
        image: scan_lines.image().clone(),
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

fn setup_line_visualization<T: CameraLocation>(dbg: DebugContext) {
    dbg.log_static(
        T::make_entity_image_path("lines"),
        &rerun::Color::from_rgb(255, 0, 0),
    );

    dbg.log_static(
        T::make_entity_image_path("projected_lines"),
        &rerun::Color::from_rgb(255, 0, 0),
    );
}

fn visualize_lines<T: CameraLocation>(
    dbg: DebugContext,
    lines: Res<DetectedLines<T>>,
    robot_pose: Res<RobotPose>,
) {
    dbg.log_with_cycle(
        T::make_entity_image_path("lines"),
        lines.image.cycle(),
        &rerun::LineStrips2D::new(
            lines
                .lines
                .iter()
                .map(|line| [Into::<Vec2>::into(line.start), Into::<Vec2>::into(line.end)]),
        ),
    );

    dbg.log_with_cycle(
        T::make_entity_image_path("projected_lines"),
        lines.image.cycle(),
        &rerun::LineStrips3D::new(lines.projected_lines.iter().map(|line| {
            [
                Into::<Vec3>::into(line.start),
                Into::<Vec3>::into(line.start),
            ]
        })),
    );

    let transform = robot_pose.as_3d();

    dbg.log_with_cycle(
        T::make_entity_image_path("projected_lines"),
        lines.image.cycle(),
        &rerun::Transform3D::from_translation(Into::<Vec3>::into(transform.translation))
            .with_quaternion(Into::<Quat>::into(transform.rotation)),
    );
}

pub fn detect_lines<T: CameraLocation>(
    mut commands: Commands,
    previous_lines: Option<Res<DetectedLines<T>>>,
    scan_lines: Res<ScanLines<T>>,
    matrix: Res<CameraMatrix<T>>,
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
                .unwrap_or(DetectedLines::empty(scan_lines.image().clone()));
            let scan_lines = scan_lines.deref().clone();
            let matrix = matrix.deref().clone();

            async move {
                Some(create_line_detection_data(
                    previous_lines,
                    line_spots,
                    scan_lines,
                    &matrix,
                ))
            }
        });
}
