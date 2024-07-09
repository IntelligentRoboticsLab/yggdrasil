use crate::{
    core::{config::layout::LayoutConfig, debug::DebugContext},
    prelude::*,
    vision::camera::matrix::CameraMatrices,
};

use heimdall::{CameraMatrix, YuyvImage};
use nalgebra::point;
use nidhogg::types::color;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::camera::{Image, TopImage};

const BALL_RADIUS: f32 = 2.0;

/// The minimum pixel distance between two neighboring scan lines.
const MIN_STEP_SIZE: i32 = 12;

/// The minimum number of scan lines for low resolution.
const MIN_NUM_OF_LOW_RES_SCAN_LINES: i32 = 25;

/// The ratio of field line width that is sampled when scanning the image.
const LINE_WIDTH_RATIO: f32 = 0.9;

/// The ratio of ball width that is sampled when scanning the image.
const BALL_WIDTH_RATIO: f32 = 0.8;

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`ScanGrid`]
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<ScanGrid>()?
            .add_system(update_scan_grid))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ScanLinesConfig {
    horizontal_scan_line_interval: usize,
    vertical_scan_line_interval: usize,
}

/// The classified color of a scan-line pixel.
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PixelColor {
    White,
    Black,
    Green,
    Unknown,
}

pub struct FieldColorApproximate {
    pub luminance: f32,
    pub hue: f32,
    pub saturation: f32,
}

#[derive(Debug)]
pub struct Line {
    pub x: i32,
    pub y_max: i32,
    pub max_index: usize,
}

#[derive(Default, Debug)]
pub struct ScanGrid {
    /// All possible y coordinates of pixels to be scanned.
    pub y: Vec<usize>,
    /// Description of all scan lines
    pub lines: Vec<Line>,
    /// Upper bound for all scan lines (exclusive)
    pub field_limit: i32,
    /// First index of the low res grid
    pub low_res_start: usize,
    /// Steps between low res grid lines
    pub low_res_step: usize,
}

const FIELD_APPROXIMATION_STEP_SIZE: usize = 8;

pub fn approximate_field_color(image: &YuyvImage) -> FieldColorApproximate {
    let height = image.height();

    let rows_to_check = [
        image.row(height * 3 / 8),
        image.row(height / 4),
        image.row(height / 8),
    ];

    let mut luminances = Vec::new();
    let mut hues = Vec::new();
    let mut saturations = Vec::new();

    for row in rows_to_check {
        for pixel in row.step_by(FIELD_APPROXIMATION_STEP_SIZE) {
            let (y, h, s2) = pixel.to_yhs2();

            luminances.push(y);
            hues.push(h);
            saturations.push(s2);
        }
    }

    let luminance = luminances.iter().sum::<f32>() / luminances.len() as f32;
    let hue = hues.iter().sum::<f32>() / hues.len() as f32;
    let saturation = saturations.iter().sum::<f32>() / saturations.len() as f32;

    FieldColorApproximate {
        luminance,
        hue,
        saturation,
    }
}

#[allow(dead_code)]
fn vertical_scan_lines(
    image: &YuyvImage,
    _scan_grid: &ScanGrid,
    _field_color: FieldColorApproximate,
) {
    for row in image.row_iter() {
        for pixel in row {
            let (_y, _h, _s) = pixel.to_yhs2();
        }
    }
}

#[system]
pub fn update_scan_grid(
    scan_grid: &mut ScanGrid,
    camera_matrix: &CameraMatrices,
    layout: &LayoutConfig,
    image: &TopImage,
    dbg: &DebugContext,
    // image: &YuyvImage,
) -> Result<()> {
    if let Some(new_scan_grid) = get_scan_grid(&camera_matrix.top, layout, image.yuyv_image()) {
        // println!("Updated scan grid: {:#?}", new_scan_grid);
        *scan_grid = new_scan_grid;

        let now = std::time::Instant::now();
        debug_scan_grid(scan_grid, image, dbg)?;
        println!("scan_lines took: {:#?}", now.elapsed());
    } else {
        warn!("Failed to update scan grid")
    };

    Ok(())
}

fn debug_scan_grid(scan_grid: &ScanGrid, image: &Image, dbg: &DebugContext) -> Result<()> {
    let mut points = Vec::new();

    for line in &scan_grid.lines {
        for y in scan_grid.y.iter().take_while(|y| **y < line.y_max as usize) {
            points.push((line.x as f32, *y as f32));
        }
    }

    dbg.log_points2d_for_image("top_camera/image/scan_grid", &points, image, color::u8::RED)?;

    Ok(())
}

fn get_scan_grid(
    camera_matrix: &CameraMatrix,
    layout: &LayoutConfig,
    image: &YuyvImage,
) -> Option<ScanGrid> {
    // println!();

    let field_diagonal = layout.field.diagonal().norm();

    // Pixel coordinates of the field diagonal
    let point_in_image = camera_matrix
        .ground_to_pixel(point![field_diagonal, 0.0, 0.0])
        .ok()?;

    let field_limit = point_in_image.y.max(-1.0) as i32;
    if field_limit >= image.height() as i32 {
        warn!("Field limit is out of bounds");
        return None;
    }

    // Field coordinates of bottom left pixel (robot frame)
    let bottom_left = camera_matrix
        .pixel_to_ground(point![0.0, image.height() as f32], 0.0)
        .inspect_err(|_| {
            warn!("No bottom left");
        })
        .ok()?
        .xy();

    // Field coordinates of bottom right pixel (robot frame)
    let bottom_right = camera_matrix
        .pixel_to_ground(point![image.width() as f32, image.height() as f32], 0.0)
        .inspect_err(|_| {
            warn!("No bottom right");
        })
        .ok()?
        .xy();

    // println!("Field diagonal: {:#?}", field_diagonal);
    // println!("Field limit: {:#?}", field_limit);
    // println!("Bottom left: {:#?}", bottom_left);
    // println!("Bottom right: {:#?}", bottom_right);
    // println!("norm {:#?}", (bottom_left - bottom_right).norm());

    let x_step_upper_bound = image.width() as i32 / MIN_NUM_OF_LOW_RES_SCAN_LINES;
    let max_x_step = {
        x_step_upper_bound.min(
            ((image.width() as f32 * BALL_RADIUS * 2.0 * BALL_WIDTH_RATIO)
                / (bottom_left - bottom_right).norm()) as i32,
        )
    };

    let mut point_on_field = (bottom_left.coords + bottom_right.coords) / 2.0;

    let mut scangrid_ys = Vec::with_capacity(image.height());
    let field_step = layout.field.line_width * LINE_WIDTH_RATIO;
    let mut y = image.height() as i32 - 1;
    let mut single_steps = false;

    while y > field_limit {
        scangrid_ys.push(y as usize);

        if single_steps {
            y -= 1;
        } else {
            point_on_field.x += field_step;

            let Ok(point_in_image) =
                camera_matrix.ground_to_pixel(point![point_on_field.x, point_on_field.y, 0.0])
            else {
                break;
            };

            let y2 = y;
            y = (y2 - 1).min((point_in_image.y + 0.5) as i32);
            single_steps = y2 - 1 == y;
        }
    }

    if y < 0 && !scangrid_ys.is_empty() && scangrid_ys.last() != Some(&0) {
        scangrid_ys.push(0);
    }

    // println!("Scangrid ys: {:#?}", scangrid_ys);

    let top_left = camera_matrix.pixel_to_ground(point![0.0, 0.0], 0.0);

    // println!("Top left ok: {:#?}", top_left.is_ok());

    let top_right = camera_matrix.pixel_to_ground(point![image.width() as f32, 0.0], 0.0);

    // println!("Top right ok: {:#?}", top_right.is_ok());

    let mut min_x_step = MIN_STEP_SIZE;

    if let (Ok(top_left), Ok(top_right)) = (top_left, top_right) {
        min_x_step = min_x_step.max(
            (image.width() as f32 * BALL_RADIUS * 2.0 * BALL_WIDTH_RATIO
                / (top_left - top_right).norm()) as i32,
        );
    }

    min_x_step = min_x_step.min(x_step_upper_bound);

    // Determine a max step size that fulfills maxXStep2 = minXStep * 2^n, maxXStep2 <= maxXStep.
    // Also compute lower y coordinates for the different lengths of scan lines.
    let mut max_x_step2 = min_x_step;
    let mut y_starts = Vec::new();

    while max_x_step2 * 2 <= max_x_step {
        let distance = get_distance_by_size(
            &camera_matrix,
            BALL_RADIUS * BALL_WIDTH_RATIO,
            max_x_step2 as f32,
        );

        let point_in_image = camera_matrix
            .ground_to_pixel(point![distance, 0.0, 0.0])
            .inspect_err(|_| {
                warn!("No point in image: distance: {:#?}", distance);
            })
            .ok()?;

        y_starts.push((point_in_image.y + 0.5) as i32);
        max_x_step2 *= 2;
    }

    y_starts.push(image.height() as i32);

    // Determine a pattern with the different lengths of scan lines, in which the longest appears once,
    // the second longest twice, etc. The pattern starts with the longest.
    let mut y_starts2 = vec![0; max_x_step2 as usize / min_x_step as usize];

    let mut step = 1;
    for y1 in y_starts.iter() {
        for y2 in y_starts2.iter_mut().step_by(step) {
            *y2 = *y1;
        }
        step *= 2;
    }

    // Initialize the scan states and the regions.
    let (width, height) = (image.width() as i32, image.height() as i32);
    let x_start = width % (width / min_x_step - 1) / 2;
    let mut i = y_starts2.len() / 2;

    let mut lines = Vec::with_capacity((width - x_start) as usize / min_x_step as usize);

    for x in (x_start..width).step_by(min_x_step as usize) {
        let y_max = y_starts2[i].min(height).max(0);

        i = (i + 1) % y_starts2.len();

        let max_index = scangrid_ys
            .iter()
            .position(|&y| y < y_max as usize)
            .unwrap_or(scangrid_ys.len());

        lines.push(Line {
            x,
            y_max,
            max_index,
        });
    }

    let low_res_step = max_x_step2 as usize / min_x_step as usize;
    let low_res_start = low_res_step / 2;

    Some(ScanGrid {
        y: scangrid_ys,
        lines,
        field_limit,
        low_res_start,
        low_res_step,
    })
}

// TODO: need a better camera matrix/projection submodule
fn get_distance_by_size(
    camera_info: &CameraMatrix,
    size_in_reality: f32,
    size_in_pixels: f32,
) -> f32 {
    // println!("camera_info mean: {:#?}", camera_info.focal_lengths.mean());
    // println!("size_in_reality: {:#?}", size_in_reality);
    // println!("size_in_pixels: {:#?}", size_in_pixels);

    let x_factor = camera_info.focal_lengths.mean();
    size_in_reality * x_factor / (size_in_pixels + f32::MIN_POSITIVE)
}
