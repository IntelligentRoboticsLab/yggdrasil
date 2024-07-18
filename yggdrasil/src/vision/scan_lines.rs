use std::{ops::Deref, sync::Arc};

use crate::{core::debug::DebugContext, nao::Cycle, prelude::*};

use super::{
    camera::{BottomImage, Image, TopImage},
    color,
    field_boundary::FieldBoundary,
    scan_grid::{BottomScanGrid, FieldColorApproximate, ScanGrid, TopScanGrid},
};

use heimdall::{YuvPixel, YuyvImage};
use nalgebra::Point2;
use nidhogg::types::{color::u8, RgbU8};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanLinesConfig {
    /// Minimum luminance delta for there to be considered an edge between pixels.
    min_edge_luminance_difference: f32,
    /// Field color
    max_field_luminance: f32,
    min_field_saturation: f32,
    min_field_hue: f32,
    max_field_hue: f32,
    /// White color
    min_white_luminance: f32,
    max_white_saturation: f32,
    /// Black color
    max_black_luminance: f32,
    max_black_saturation: f32,
}

impl Config for ScanLinesConfig {
    const PATH: &'static str = "scan_lines.toml";
}

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`TopScanLines`]
/// - [`BottomScanLines`]
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.init_config::<ScanLinesConfig>()?
            .add_system(scan_lines_system.after(super::scan_grid::update_scan_grid))
            .add_startup_system(init_scan_lines)
    }
}

/// Horizontal and vertical scanlines for an image.
#[derive(Clone)]
pub struct ScanLines {
    image: Image,
    horizontal: Arc<ScanLine>,
    vertical: Arc<ScanLine>,
}

impl ScanLines {
    pub fn new(image: Image, horizontal: ScanLine, vertical: ScanLine) -> Self {
        Self {
            image,
            horizontal: Arc::new(horizontal),
            vertical: Arc::new(vertical),
        }
    }

    pub fn image(&self) -> &Image {
        &self.image
    }

    pub fn horizontal(&self) -> &ScanLine {
        &self.horizontal
    }

    pub fn vertical(&self) -> &ScanLine {
        &self.vertical
    }
}

/// A set of classified scanline regions.
#[derive(Debug, Default)]
pub struct ScanLine {
    raw: Vec<ClassifiedScanLineRegion>,
}

impl ScanLine {
    pub fn new(raw: Vec<ClassifiedScanLineRegion>) -> Self {
        Self { raw }
    }

    pub fn regions(&self) -> std::slice::Iter<ClassifiedScanLineRegion> {
        self.raw.iter()
    }

    /// Iterate over the line spots in these scanlines.
    ///
    /// A line spot is the point in the middle of a *white* scanline region.
    pub fn line_spots(&self) -> impl Iterator<Item = Point2<f32>> + '_ {
        self.raw
            .iter()
            .filter(|r| matches!(r.color, RegionColor::WhiteOrBlack))
            .map(|r| r.line.region.line_spot())
    }

    pub fn classified_scan_line_regions(&self) -> &[ClassifiedScanLineRegion] {
        &self.raw
    }
}

/// Scanlines for the top camera.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct TopScanLines(ScanLines);

/// Scanlines for the bottom camera.
#[derive(derive_more::Deref, derive_more::DerefMut)]
pub struct BottomScanLines(ScanLines);

#[derive(Debug)]
pub struct ScanLineRegion {
    region: Region,
    approx_color: YuvPixel,
}

impl Deref for ScanLineRegion {
    type Target = Region;

    fn deref(&self) -> &Self::Target {
        &self.region
    }
}

impl ScanLineRegion {
    /// Approximated color of the region.
    pub fn approx_color(&self) -> &YuvPixel {
        &self.approx_color
    }

    /// Using a color sample and a weight, update the approximated color of the region.
    fn add_sample(&mut self, sample: YuvPixel, weight: usize) {
        let self_weight = self.region.length();

        let (y, u, v) = (
            self.approx_color.y as f32,
            self.approx_color.u as f32,
            self.approx_color.v as f32,
        );

        let (y_sample, u_sample, v_sample) = (sample.y as f32, sample.u as f32, sample.v as f32);

        let y_sum = y * self_weight as f32 + y_sample * weight as f32;
        let u_sum = u * self_weight as f32 + u_sample * weight as f32;
        let v_sum = v * self_weight as f32 + v_sample * weight as f32;

        let total = self_weight as f32 + weight as f32;

        self.approx_color = YuvPixel {
            y: (y_sum / total) as u8,
            u: (u_sum / total) as u8,
            v: (v_sum / total) as u8,
        };
    }

    /// Classify the region color based on the approximate color.
    fn classify(
        self,
        config: &ScanLinesConfig,
        field: &FieldColorApproximate,
    ) -> ClassifiedScanLineRegion {
        let color = RegionColor::classify_yuv_pixel(config, field, self.approx_color.clone());

        ClassifiedScanLineRegion { line: self, color }
    }

    pub fn region(&self) -> &Region {
        &self.region
    }
}

/// Scanline region with a classified color.
#[derive(Debug)]
pub struct ClassifiedScanLineRegion {
    line: ScanLineRegion,
    color: RegionColor,
}

impl Deref for ClassifiedScanLineRegion {
    type Target = ScanLineRegion;

    fn deref(&self) -> &Self::Target {
        &self.line
    }
}

impl ClassifiedScanLineRegion {
    /// Merges adjacent regions with the same color.
    pub fn simplify(regions: Vec<Self>) -> Vec<Self> {
        let mut new_regions = Vec::new();

        let mut current_region = None;
        for region in regions {
            let Some(mut curr) = current_region.take() else {
                current_region = Some(region);
                continue;
            };

            let same_fixed_point =
                region.line.region.fixed_point() == curr.line.region.fixed_point();
            let same_color = region.color == curr.color;

            if same_fixed_point && same_color {
                // merge scan lines
                curr.line
                    .region
                    .set_end_point(region.line.region.end_point());

                let weight = curr.line.region.length();
                curr.line
                    .add_sample(region.line.approx_color.clone(), weight);

                current_region = Some(curr);
            } else {
                new_regions.push(curr);
                current_region = Some(region);
            }
        }

        if let Some(curr) = current_region.take() {
            new_regions.push(curr);
        }

        new_regions
    }

    pub fn scan_line_region(&self) -> &ScanLineRegion {
        &self.line
    }

    pub fn color(&self) -> &RegionColor {
        &self.color
    }
}

#[derive(Debug)]
pub enum Region {
    Vertical {
        x: usize,
        y_start: usize,
        y_end: usize,
    },
    Horizontal {
        y: usize,
        x_start: usize,
        x_end: usize,
    },
}

impl Region {
    pub fn start_point(&self) -> usize {
        match self {
            Region::Vertical { y_start, .. } => *y_start,
            Region::Horizontal { x_start, .. } => *x_start,
        }
    }

    pub fn end_point(&self) -> usize {
        match self {
            Region::Vertical { y_end, .. } => *y_end,
            Region::Horizontal { x_end, .. } => *x_end,
        }
    }

    fn set_end_point(&mut self, end_point: usize) {
        match self {
            Region::Vertical { y_end, .. } => *y_end = end_point,
            Region::Horizontal { x_end, .. } => *x_end = end_point,
        }
    }

    pub fn fixed_point(&self) -> usize {
        match self {
            Region::Vertical { x, .. } => *x,
            Region::Horizontal { y, .. } => *y,
        }
    }

    /// Get the position of the corresponding line spot
    ///
    /// The line spot is the point in the middle of the region.
    pub fn line_spot(&self) -> Point2<f32> {
        match self {
            Region::Vertical { x, y_start, y_end } => {
                Point2::new(*x as f32, 0.5 * (y_start + y_end) as f32)
            }
            Region::Horizontal { y, x_start, x_end } => {
                Point2::new(0.5 * (x_start + x_end) as f32, *y as f32)
            }
        }
    }

    pub fn direction(&self) -> Direction {
        match self {
            Region::Vertical { .. } => Direction::Vertical,
            Region::Horizontal { .. } => Direction::Horizontal,
        }
    }

    pub fn length(&self) -> usize {
        self.end_point() - self.start_point()
    }
}

pub enum Direction {
    Horizontal,
    Vertical,
}

fn get_horizontal_scan_lines(
    config: &ScanLinesConfig,
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    camera: CameraType,
) -> ScanLine {
    let mut regions = Vec::new();

    for y in &scan_grid.y {
        let mut current_region = None;
        for line in &scan_grid.lines {
            let x = line.x as usize;
            let y = *y;

            if camera == CameraType::Top {
                // skip lines below the field boundary
                let boundary = field_boundary.height_at_pixel(x as f32) as usize;
                if y < boundary {
                    continue;
                }
            }

            let pixels = unsafe {
                [
                    yuyv.pixel_unchecked(x - 1, y),
                    yuyv.pixel_unchecked(x, y),
                    yuyv.pixel_unchecked(x + 1, y),
                ]
            };

            let pixel = YuvPixel::average(&pixels);

            let Some(curr_region) = &mut current_region else {
                // first region of this y coordinate
                current_region = Some(ScanLineRegion {
                    region: Region::Horizontal {
                        y,
                        x_start: x,
                        x_end: x,
                    },
                    approx_color: pixel,
                });
                continue;
            };

            let lum_diff = (pixel.y as f32 - curr_region.approx_color.y as f32).abs();

            if lum_diff >= config.min_edge_luminance_difference {
                // find the exact pixel where the largest difference is
                let x_edge = find_edge(
                    yuyv,
                    curr_region.region.end_point(),
                    x,
                    y,
                    Direction::Horizontal,
                );

                curr_region.region.set_end_point(x_edge);

                // create new region starting from the edge
                let mut new_region = ScanLineRegion {
                    region: Region::Horizontal {
                        y,
                        x_start: x_edge,
                        x_end: x,
                    },
                    approx_color: pixel,
                };

                // put new region in place of curr_region
                std::mem::swap(curr_region, &mut new_region);
                // and push the old region to the vec
                regions.push(new_region.classify(config, field));

                continue;
            } else {
                // get the length of the region inbetween
                let weight = x - curr_region.region.end_point();

                // set the end point to the current x
                curr_region.region.set_end_point(x);

                // add the pixel color sample to the region with a weight of the added length
                curr_region.add_sample(pixel, weight);
            }
        }

        if let Some(curr_region) = current_region.take() {
            regions.push(curr_region.classify(config, field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

fn get_vertical_scan_lines(
    config: &ScanLinesConfig,
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    camera: CameraType,
) -> ScanLine {
    let mut regions = Vec::new();

    for line in &scan_grid.lines {
        let mut current_region = None;

        // take the y coordinates of the scan grid, skipping the first and last line
        for y in scan_grid
            .y
            .iter()
            .skip(1)
            .take(scan_grid.y.len().saturating_sub(2))
        {
            let x = line.x as usize;
            let y = *y;

            if camera == CameraType::Top {
                // skip lines above the field boundary
                let boundary = field_boundary.height_at_pixel(x as f32) as usize;
                if y < boundary {
                    continue;
                }
            }

            let pixels = unsafe {
                [
                    yuyv.pixel_unchecked(x, y - 1),
                    yuyv.pixel_unchecked(x, y),
                    yuyv.pixel_unchecked(x, y + 1),
                ]
            };
            let pixel = YuvPixel::average(&pixels);

            let Some(curr_region) = &mut current_region else {
                // first region of this y coordinate
                current_region = Some(ScanLineRegion {
                    region: Region::Vertical {
                        x,
                        y_start: y,
                        y_end: y,
                    },
                    approx_color: pixel,
                });
                continue;
            };

            let lum_diff = (pixel.y as f32 - curr_region.approx_color.y as f32).abs();

            if lum_diff >= config.min_edge_luminance_difference {
                // find the exact pixel where the largest difference is
                let y_edge = find_edge(
                    yuyv,
                    curr_region.region.end_point(),
                    y,
                    x,
                    Direction::Vertical,
                );

                curr_region.region.set_end_point(y_edge);

                // create new region starting from the edge
                let mut new_region = ScanLineRegion {
                    region: Region::Vertical {
                        x,
                        y_start: y_edge,
                        y_end: y,
                    },
                    approx_color: pixel,
                };

                // put new region in place of curr_region
                std::mem::swap(curr_region, &mut new_region);
                // and push the old region to the vec
                regions.push(new_region.classify(config, field));

                continue;
            } else {
                // get the length of the region inbetween
                let weight = y - curr_region.region.end_point();

                // set the end point to the current y
                curr_region.region.set_end_point(y);

                // add the pixel color sample to the region with a weight of the added length
                curr_region.add_sample(pixel, weight);
            }
        }

        if let Some(curr_region) = current_region.take() {
            regions.push(curr_region.classify(config, field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

fn get_scan_lines(
    config: &ScanLinesConfig,
    image: Image,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    camera: CameraType,
) -> ScanLines {
    let yuyv = image.yuyv_image();

    let field = FieldColorApproximate::new(yuyv);

    let horizontal =
        get_horizontal_scan_lines(config, &field, yuyv, scan_grid, field_boundary, camera);
    let vertical = get_vertical_scan_lines(config, &field, yuyv, scan_grid, field_boundary, camera);

    ScanLines::new(image, horizontal, vertical)
}

#[system]
pub fn scan_lines_system(
    config: &ScanLinesConfig,
    (top_scan_lines, bottom_scan_lines): (&mut TopScanLines, &mut BottomScanLines),
    (top_image, bottom_image): (&TopImage, &BottomImage),
    (top_scan_grid, bottom_scan_grid): (&TopScanGrid, &BottomScanGrid),
    field_boundary: &FieldBoundary,
    curr_cycle: &Cycle,
    dbg: &DebugContext,
) -> Result<()> {
    update_scan_lines(
        config,
        top_scan_lines,
        top_image,
        top_scan_grid,
        field_boundary,
        curr_cycle,
        dbg,
        CameraType::Top,
    )?;

    update_scan_lines(
        config,
        bottom_scan_lines,
        bottom_image,
        bottom_scan_grid,
        field_boundary,
        curr_cycle,
        dbg,
        CameraType::Bottom,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_scan_lines(
    config: &ScanLinesConfig,
    scan_lines: &mut ScanLines,
    image: &Image,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    curr_cycle: &Cycle,
    dbg: &DebugContext,
    camera: CameraType,
) -> Result<()> {
    if !scan_grid.image.is_from_cycle(*curr_cycle) || scan_grid.lines.is_empty() {
        return Ok(());
    }

    let new = get_scan_lines(config, image.clone(), scan_grid, field_boundary, camera);

    debug_scan_lines(&new.horizontal, dbg, image, camera)?;
    debug_scan_lines(&new.vertical, dbg, image, camera)?;

    debug_scan_line_spots(&new.horizontal, dbg, image, u8::RED, camera)?;
    debug_scan_line_spots(&new.vertical, dbg, image, u8::BLUE, camera)?;

    *scan_lines = new;

    Ok(())
}

/// Find the edge of a region in a scanline.
///
/// The edge is the pixel where the luminance difference between the current pixel and the next pixel is the largest.
fn find_edge(
    yuyv: &YuyvImage,
    start: usize,
    end: usize,
    fixed: usize,
    direction: Direction,
) -> usize {
    let (pos_edge, _value_edge) =
        (start..=end).fold((start, 0.0), |(pos_edge, diff_edge), pos_next| {
            // get the current pixel and next pixels
            let (pixel, pixel_next) = match direction {
                Direction::Horizontal => unsafe {
                    (
                        yuyv.pixel_unchecked(pos_next, fixed),
                        yuyv.pixel_unchecked(pos_next + 1, fixed),
                    )
                },
                Direction::Vertical => unsafe {
                    (
                        yuyv.pixel_unchecked(fixed, pos_next),
                        yuyv.pixel_unchecked(fixed, pos_next + 1),
                    )
                },
            };

            let lum = pixel.y as f32;
            let lum_next = pixel_next.y as f32;

            let next_diff = (lum_next - lum).abs();

            match next_diff > diff_edge {
                true => (pos_next, next_diff),
                false => (pos_edge, diff_edge),
            }
        });

    pos_edge
}

/// The classified color of a scan-line region.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegionColor {
    WhiteOrBlack,
    Green,
    Unknown,
}

impl RegionColor {
    // TODO: use our field color approximate
    pub fn classify_yuv_pixel(
        config: &ScanLinesConfig,
        _field: &FieldColorApproximate,
        pixel: YuvPixel,
    ) -> Self {
        let yhs = pixel.to_yhs2();

        if Self::is_green(config, yhs) {
            return RegionColor::Green;
        }

        if Self::is_white(config, yhs) {
            return RegionColor::WhiteOrBlack;
        }

        if Self::is_black(config, yhs) {
            // We mark black spots as white regions for ball detection
            return RegionColor::WhiteOrBlack;
        }

        RegionColor::Unknown
    }

    fn is_green(config: &ScanLinesConfig, (y, h, s): (f32, f32, f32)) -> bool {
        y <= config.max_field_luminance
            && s >= config.min_field_saturation
            && (config.min_field_hue..=config.max_field_hue).contains(&h)
    }

    fn is_white(config: &ScanLinesConfig, (y, _h, s): (f32, f32, f32)) -> bool {
        y >= config.min_white_luminance && s <= config.max_white_saturation
    }

    fn is_black(config: &ScanLinesConfig, (y, _h, s): (f32, f32, f32)) -> bool {
        y <= config.max_black_luminance && s <= config.max_black_saturation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraType {
    Top,
    Bottom,
}

fn debug_scan_lines(
    scan_line: &ScanLine,
    dbg: &DebugContext,
    image: &Image,
    camera: CameraType,
) -> Result<()> {
    let scan_line = &scan_line.raw;

    if scan_line.is_empty() {
        return Ok(());
    }

    let direction = scan_line[0].line.region.direction();

    let region_len = scan_line.len();

    let mut lines = Vec::with_capacity(region_len);
    let mut colors = Vec::with_capacity(region_len);
    let mut classifications = Vec::with_capacity(region_len);

    for line in scan_line {
        let (r, g, b) = color::yuv_to_rgb_bt601((
            line.line.approx_color.y,
            line.line.approx_color.u,
            line.line.approx_color.v,
        ));

        colors.push(RgbU8::new(r, g, b));

        let (r, g, b) = match line.color {
            RegionColor::WhiteOrBlack => (255, 255, 255),
            RegionColor::Green => (0, 255, 0),
            RegionColor::Unknown => (128, 128, 128),
        };

        let start = line.line.region.start_point() as f32;
        let end = line.line.region.end_point() as f32;
        let fixed = line.line.region.fixed_point() as f32;

        match direction {
            Direction::Horizontal => lines.push([(start, fixed), (end, fixed)]),
            Direction::Vertical => lines.push([(fixed, start), (fixed, end)]),
        }
        classifications.push(RgbU8::new(r, g, b));
    }

    let direction_str = match direction {
        Direction::Horizontal => "horizontal",
        Direction::Vertical => "vertical",
    };

    let camera_str = match camera {
        CameraType::Top => "top",
        CameraType::Bottom => "bottom",
    };

    dbg.log_lines2d_for_image_with_colors(
        format!("{camera_str}_camera/image/scan_lines/approximates/{direction_str}"),
        &lines,
        image,
        &colors,
    )?;

    dbg.log_lines2d_for_image_with_colors(
        format!("{camera_str}_camera/image/scan_lines/classifications/{direction_str}"),
        &lines,
        image,
        &classifications,
    )?;

    Ok(())
}

fn debug_scan_line_spots(
    scan_line: &ScanLine,
    dbg: &DebugContext,
    image: &Image,
    color: RgbU8,
    camera: CameraType,
) -> Result<()> {
    let regions = &scan_line.raw;

    if regions.is_empty() {
        return Ok(());
    }

    let direction = regions[0].line.region.direction();

    let line_spots = scan_line
        .line_spots()
        .map(|s| (s.x, s.y))
        .collect::<Vec<_>>();

    let colors = vec![color; line_spots.len()];

    let direction_str = match direction {
        Direction::Horizontal => "horizontal",
        Direction::Vertical => "vertical",
    };

    let camera_str = match camera {
        CameraType::Top => "top",
        CameraType::Bottom => "bottom",
    };

    dbg.log_points2d_for_image_with_colors(
        format!("{camera_str}_camera/image/scan_lines/spots/{direction_str}"),
        &line_spots,
        image,
        &colors,
    )?;

    Ok(())
}

#[startup_system]
fn init_scan_lines(
    storage: &mut Storage,
    config: &ScanLinesConfig,
    (top_image, bottom_image): (&TopImage, &BottomImage),
    (top_scan_grid, bottom_scan_grid): (&mut TopScanGrid, &mut BottomScanGrid),
    field_boundary: &FieldBoundary,
) -> Result<()> {
    let top = get_scan_lines(
        config,
        top_image.deref().clone(),
        top_scan_grid,
        field_boundary,
        CameraType::Top,
    );

    let bottom = get_scan_lines(
        config,
        bottom_image.deref().clone(),
        bottom_scan_grid,
        field_boundary,
        CameraType::Bottom,
    );

    storage.add_resource(Resource::new(TopScanLines(top)))?;
    storage.add_resource(Resource::new(BottomScanLines(bottom)))?;

    Ok(())
}
