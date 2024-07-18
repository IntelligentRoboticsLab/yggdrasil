use std::ops::Deref;

use crate::{
    core::{config::layout::LayoutConfig, debug::DebugContext},
    prelude::*,
    vision::camera::matrix::CameraMatrices,
};

use heimdall::{CameraMatrix, YuyvImage};
use nalgebra::point;
use nidhogg::types::color;

use super::{
    camera::{BottomImage, Image, TopImage},
    scan_lines::CameraType,
};

/// The step size for approximating the field color.
const FIELD_APPROXIMATION_STEP_SIZE: usize = 8;

/// The number of brightest pixels to approximate the white color.
const FIELD_APPROXIMATION_WHITE_TOP_K: usize = 10;

/// The radius of the ball in cm.
const BALL_RADIUS: f32 = 2.0;

/// The minimum pixel distance between two neighboring scan lines.
const MIN_STEP_SIZE: i32 = 12;

/// The minimum number of scan lines for low resolution.
const MIN_NUM_OF_LOW_RES_SCAN_LINES: i32 = 25;

/// The ratio of field line width that is sampled when scanning the image.
const LINE_WIDTH_RATIO: f32 = 0.9;

/// The ratio of ball width that is sampled when scanning the image.
const BALL_WIDTH_RATIO: f32 = 0.8;

/// Module that generates a scan grid from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`TopScanGrid`]: The scan grid for the top camera.
/// - [`BottomScanGrid`]: The scan grid for the bottom camera.
pub struct ScanGridModule;

impl Module for ScanGridModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(update_scan_grid.after(super::camera::camera_system))
            .add_startup_system(init_scan_grid)
    }
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

/// Approximate color values of the field.
///
/// The color is approximated by the mean and standard deviation of the luminance, hue, and saturation.
/// The white color is also approximated by the mean and standard deviation of the luminance of the 10 brightest pixels.
#[derive(Debug, Clone)]
pub struct FieldColorApproximate {
    pub luminance: (f32, f32),
    pub hue: (f32, f32),
    pub saturation: (f32, f32),
    pub white: (f32, f32),
}

impl FieldColorApproximate {
    pub fn new(image: &YuyvImage) -> Self {
        let height = image.height();

        let rows_to_check = [
            image.row(height - height * 3 / 8),
            image.row(height - height / 4),
            image.row(height - height / 8),
        ];

        let mut luminances = Vec::new();
        let mut hues = Vec::new();
        let mut saturations = Vec::new();

        for row in rows_to_check.into_iter().flatten() {
            for pixel in row.step_by(FIELD_APPROXIMATION_STEP_SIZE) {
                let (y, h, s2) = pixel.to_yhs2();

                luminances.push(y);
                hues.push(h);
                saturations.push(s2);
            }
        }

        let luminance = mean_and_std(&luminances);
        let hue = mean_and_std(&hues);
        let saturation = mean_and_std(&saturations);

        luminances.sort_by(|a, b| a.total_cmp(b).reverse());
        let white = mean_and_std(&luminances[..FIELD_APPROXIMATION_WHITE_TOP_K]);

        Self {
            luminance,
            hue,
            saturation,
            white,
        }
    }
}

fn mean(data: &[f32]) -> f32 {
    let sum = data.iter().sum::<f32>();
    let count = data.len();

    sum / count as f32
}

fn mean_and_std(data: &[f32]) -> (f32, f32) {
    let (mean, count) = (mean(data), data.len());

    let variance = data
        .iter()
        .map(|value| {
            let diff = mean - (*value);

            diff * diff
        })
        .sum::<f32>()
        / count as f32;

    (mean, variance.sqrt())
}

#[derive(Debug)]
pub struct Line {
    pub x: i32,
    pub y_max: i32,
    pub max_index: usize,
}

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct TopScanGrid(ScanGrid);

#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct BottomScanGrid(ScanGrid);

pub struct ScanGrid {
    pub image: Image,
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

#[startup_system]
pub fn init_scan_grid(
    storage: &mut Storage,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    let top_image = top_image.deref().clone();
    let bottom_image = bottom_image.deref().clone();

    let top = ScanGrid {
        image: top_image,
        y: Vec::new(),
        lines: Vec::new(),
        field_limit: 0,
        low_res_start: 0,
        low_res_step: 0,
    };

    let bottom = get_bottom_scan_grid(&bottom_image);

    storage.add_resource(Resource::new(TopScanGrid(top)))?;
    storage.add_resource(Resource::new(BottomScanGrid(bottom)))?;

    Ok(())
}

#[system]
pub fn update_scan_grid(
    (top_scan_grid, bottom_scan_grid): (&mut TopScanGrid, &mut BottomScanGrid),
    camera_matrix: &CameraMatrices,
    layout: &LayoutConfig,
    top_image: &TopImage,
    bottom_image: &BottomImage,
    dbg: &DebugContext,
) -> Result<()> {
    update_top_scan_grid(top_scan_grid, camera_matrix, layout, top_image, dbg)?;
    update_bottom_scan_grid(bottom_scan_grid, bottom_image, dbg)?;

    Ok(())
}

pub fn update_top_scan_grid(
    scan_grid: &mut ScanGrid,
    camera_matrix: &CameraMatrices,
    layout: &LayoutConfig,
    image: &TopImage,
    dbg: &DebugContext,
) -> Result<()> {
    if scan_grid.image.is_from_cycle(image.cycle()) {
        return Ok(());
    }

    if let Some(new_scan_grid) = get_scan_grid(&camera_matrix.top, layout, image) {
        *scan_grid = new_scan_grid;
        debug_scan_grid(scan_grid, image, dbg, CameraType::Top)?;
    };

    Ok(())
}

pub fn update_bottom_scan_grid(
    scan_grid: &mut ScanGrid,
    image: &BottomImage,
    dbg: &DebugContext,
) -> Result<()> {
    if scan_grid.image.is_from_cycle(image.cycle()) {
        return Ok(());
    }

    *scan_grid = get_bottom_scan_grid(image);
    debug_scan_grid(scan_grid, image, dbg, CameraType::Bottom)?;

    Ok(())
}

fn debug_scan_grid(
    scan_grid: &ScanGrid,
    image: &Image,
    dbg: &DebugContext,
    camera: CameraType,
) -> Result<()> {
    let mut points = Vec::new();

    for line in &scan_grid.lines {
        for y in scan_grid.y.iter() {
            points.push((line.x as f32, *y as f32));
        }
    }

    let camera_str = match camera {
        CameraType::Top => "top",
        CameraType::Bottom => "bottom",
    };

    dbg.log_points2d_for_image(
        format!("{camera_str}_camera/image/scan_lines/scan_grid"),
        &points,
        image,
        color::u8::ORANGE,
    )?;

    Ok(())
}

fn get_scan_grid(
    camera_matrix: &CameraMatrix,
    layout: &LayoutConfig,
    image: &Image,
) -> Option<ScanGrid> {
    let image = image.clone();
    let yuyv = image.yuyv_image();

    let field_diagonal = layout.field.diagonal().norm();

    // Pixel coordinates of the field diagonal
    let point_in_image = camera_matrix
        .ground_to_pixel(point![field_diagonal, 0.0, 0.0])
        .ok()?;

    let field_limit = point_in_image.y.max(-1.0) as i32;
    if field_limit >= yuyv.height() as i32 {
        return None;
    }

    // Field coordinates of bottom left pixel (robot frame)
    let bottom_left = camera_matrix
        .pixel_to_ground(point![0.0, yuyv.height() as f32], 0.0)
        .ok()?
        .xy();

    // Field coordinates of bottom right pixel (robot frame)
    let bottom_right = camera_matrix
        .pixel_to_ground(point![yuyv.width() as f32, yuyv.height() as f32], 0.0)
        .ok()?
        .xy();

    let x_step_upper_bound = yuyv.width() as i32 / MIN_NUM_OF_LOW_RES_SCAN_LINES;
    let max_x_step = {
        x_step_upper_bound.min(
            ((yuyv.width() as f32 * BALL_RADIUS * 2.0 * BALL_WIDTH_RATIO)
                / (bottom_left - bottom_right).norm()) as i32,
        )
    };

    let mut point_on_field = (bottom_left.coords + bottom_right.coords) / 2.0;

    let mut scangrid_ys = Vec::with_capacity(yuyv.height());
    let field_step = layout.field.line_width * LINE_WIDTH_RATIO;
    let mut y = yuyv.height() as i32 - 1;
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

    let top_left = camera_matrix.pixel_to_ground(point![0.0, 0.0], 0.0);

    let top_right = camera_matrix.pixel_to_ground(point![yuyv.width() as f32, 0.0], 0.0);

    let mut min_x_step = MIN_STEP_SIZE;

    if let (Ok(top_left), Ok(top_right)) = (top_left, top_right) {
        min_x_step = min_x_step.max(
            (yuyv.width() as f32 * BALL_RADIUS * 2.0 * BALL_WIDTH_RATIO
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
            camera_matrix,
            BALL_RADIUS * BALL_WIDTH_RATIO,
            max_x_step2 as f32,
        );

        let point_in_image = camera_matrix
            .ground_to_pixel(point![distance, 0.0, 0.0])
            .ok()?;

        y_starts.push((point_in_image.y + 0.5) as i32);
        max_x_step2 *= 2;
    }

    y_starts.push(yuyv.height() as i32);

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
    let (width, height) = (yuyv.width() as i32, yuyv.height() as i32);
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

    scangrid_ys.reverse();

    Some(ScanGrid {
        image,
        y: scangrid_ys,
        lines,
        field_limit,
        low_res_start,
        low_res_step,
    })
}

fn get_bottom_scan_grid(image: &Image) -> ScanGrid {
    let image = image.clone();

    const GAP_SIZE_BOTTOM: usize = 8;

    let height = image.yuyv_image().height();
    let width = image.yuyv_image().width();

    // // Get the step size after padding with (gap size)/2 pixels
    // let step_y = (height - GAP_SIZE_BOTTOM) / GAP_SIZE_BOTTOM;
    // let step_x = (width - GAP_SIZE_BOTTOM) / GAP_SIZE_BOTTOM;

    let y = (0..height)
        // pad with (gap size)/2 pixels
        .skip(GAP_SIZE_BOTTOM / 2)
        .step_by(GAP_SIZE_BOTTOM)
        .collect();

    let lines = (0..width)
        // pad with (gap size)/2 pixels
        .skip(GAP_SIZE_BOTTOM / 2)
        .step_by(GAP_SIZE_BOTTOM)
        .map(|x| Line {
            x: x as i32,
            y_max: height as i32,
            max_index: 0,
        })
        .collect();

    ScanGrid {
        image,
        y,
        lines,
        field_limit: 0,
        low_res_start: 0,
        low_res_step: 0,
    }
}

// TODO: need a better camera matrix/projection submodule
fn get_distance_by_size(
    camera_info: &CameraMatrix,
    size_in_reality: f32,
    size_in_pixels: f32,
) -> f32 {
    let x_factor = camera_info.focal_lengths.mean();
    size_in_reality * x_factor / (size_in_pixels + f32::MIN_POSITIVE)
}
