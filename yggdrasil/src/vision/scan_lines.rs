use std::{ops::Deref, sync::Arc};

use crate::{
    core::debug::{
        DebugContext,
        debug_system::{DebugAppExt, SystemToggle},
    },
    nao::Cycle,
    prelude::*,
};

use super::{
    camera::{Image, init_camera},
    color,
    field_boundary::FieldBoundary,
    scan_grid::{FieldColorApproximate, ScanGrid},
};
use bevy::prelude::*;

use heimdall::{Bottom, CameraLocation, CameraPosition, Top, YuvPixel};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};
use yggdrasil_rerun_comms::protocol::control::FieldColorConfig;

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
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
    /// Green chromaticity threshold
    pub green_chromaticity_threshold: f32,
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
}

impl Config for ScanLinesConfig {
    const PATH: &'static str = "scan_lines.toml";
}

impl From<FieldColorConfig> for ScanLinesConfig {
    fn from(value: FieldColorConfig) -> Self {
        ScanLinesConfig {
            min_edge_luminance_difference: value.min_edge_luminance_difference,
            max_field_luminance: value.max_field_luminance,
            min_field_saturation: value.min_field_saturation,
            min_field_hue: value.min_field_hue,
            max_field_hue: value.max_field_hue,
            min_white_luminance: value.min_white_luminance,
            max_white_saturation: value.max_white_saturation,
            max_black_luminance: value.max_black_luminance,
            max_black_saturation: value.max_black_saturation,
            green_chromaticity_threshold: value.green_chromaticity_threshold,
            red_chromaticity_threshold: value.red_chromaticity_threshold,
            blue_chromaticity_threshold: value.blue_chromaticity_threshold,
        }
    }
}

impl From<&ScanLinesConfig> for FieldColorConfig {
    fn from(value: &ScanLinesConfig) -> Self {
        FieldColorConfig {
            min_edge_luminance_difference: value.min_edge_luminance_difference,
            max_field_luminance: value.max_field_luminance,
            min_field_saturation: value.min_field_saturation,
            min_field_hue: value.min_field_hue,
            max_field_hue: value.max_field_hue,
            min_white_luminance: value.min_white_luminance,
            max_white_saturation: value.max_white_saturation,
            max_black_luminance: value.max_black_luminance,
            max_black_saturation: value.max_black_saturation,
            green_chromaticity_threshold: value.green_chromaticity_threshold,
            red_chromaticity_threshold: value.red_chromaticity_threshold,
            blue_chromaticity_threshold: value.blue_chromaticity_threshold,
        }
    }
}

/// Plugin that generates scan-lines from taken NAO images.
pub struct ScanLinesPlugin;

impl Plugin for ScanLinesPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<ScanLinesConfig>()
            .add_systems(
                Startup,
                (init_scan_lines::<Top>, init_scan_lines::<Bottom>)
                    .after(init_camera::<Top>)
                    .after(init_camera::<Bottom>),
            )
            .add_systems(
                Update,
                (
                    update_scan_lines::<Top>
                        .after(super::scan_grid::update_top_scan_grid)
                        .run_if(resource_exists_and_changed::<ScanGrid<Top>>),
                    update_scan_lines::<Bottom>
                        .after(super::scan_grid::update_bottom_scan_grid)
                        .run_if(resource_exists_and_changed::<ScanGrid<Bottom>>),
                ),
            )
            .add_named_debug_systems(
                PostUpdate,
                (
                    visualize_scan_lines::<Top>
                        .run_if(resource_exists_and_changed::<ScanLines<Top>>),
                    visualize_scan_lines::<Bottom>
                        .run_if(resource_exists_and_changed::<ScanLines<Bottom>>),
                ),
                "Visualize scan lines",
                SystemToggle::Disable,
            );
    }
}

/// Compute the average of three YUV pixels, without allocation.
#[inline]
fn avg_three(
    p1: heimdall::YuvPixel,
    p2: heimdall::YuvPixel,
    p3: heimdall::YuvPixel,
) -> heimdall::YuvPixel {
    let y = ((u16::from(p1.y) + u16::from(p2.y) + u16::from(p3.y) + 1) / 3) as u8;
    let u = ((u16::from(p1.u) + u16::from(p2.u) + u16::from(p3.u) + 1) / 3) as u8;
    let v = ((u16::from(p1.v) + u16::from(p2.v) + u16::from(p3.v) + 1) / 3) as u8;
    heimdall::YuvPixel { y, u, v }
}

/// Horizontal and vertical scanlines for an image.
#[derive(Resource)]
pub struct ScanLines<T: CameraLocation> {
    image: Image<T>,
    horizontal: Arc<ScanLine>,
    vertical: Arc<ScanLine>,
}

// NOTE: This needs to be implemented manually because the bounds cannot be inferred properly
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for ScanLines<T> {
    fn clone(&self) -> Self {
        Self {
            image: self.image.clone(),
            horizontal: self.horizontal.clone(),
            vertical: self.vertical.clone(),
        }
    }
}

impl<T: CameraLocation> ScanLines<T> {
    #[must_use]
    pub fn new(image: Image<T>, horizontal: ScanLine, vertical: ScanLine) -> Self {
        Self {
            image,
            horizontal: Arc::new(horizontal),
            vertical: Arc::new(vertical),
        }
    }

    #[must_use]
    pub fn image(&self) -> &Image<T> {
        &self.image
    }

    #[must_use]
    pub fn horizontal(&self) -> &ScanLine {
        &self.horizontal
    }

    #[must_use]
    pub fn vertical(&self) -> &ScanLine {
        &self.vertical
    }

    pub fn line_spots(&self) -> impl Iterator<Item = Point2<f32>> + use<'_, T> {
        self.vertical
            .line_spots()
            .chain(self.horizontal.line_spots())
    }
}

/// A set of classified scanline regions.
#[derive(Debug, Default)]
pub struct ScanLine {
    raw: Vec<ClassifiedScanLineRegion>,
}

impl ScanLine {
    #[must_use]
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

    #[must_use]
    pub fn classified_scan_line_regions(&self) -> &[ClassifiedScanLineRegion] {
        &self.raw
    }
}

#[derive(Debug, Clone)]
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
    #[must_use]
    pub fn approx_color(&self) -> &YuvPixel {
        &self.approx_color
    }

    /// Using a color sample and a weight, update the approximated color of the region.
    fn add_sample(&mut self, sample: YuvPixel, weight: usize) {
        let self_weight = self.region.length();

        let (y, u, v) = (
            f32::from(self.approx_color.y),
            f32::from(self.approx_color.u),
            f32::from(self.approx_color.v),
        );

        let (y_sample, u_sample, v_sample) = (
            f32::from(sample.y),
            f32::from(sample.u),
            f32::from(sample.v),
        );

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
        let color = RegionColor::classify_yuv_pixel(config, field, self.approx_color);

        ClassifiedScanLineRegion { line: self, color }
    }

    #[must_use]
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
    #[must_use]
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
                curr.line.add_sample(region.line.approx_color, weight);

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

    #[must_use]
    pub fn scan_line_region(&self) -> &ScanLineRegion {
        &self.line
    }

    #[must_use]
    pub fn color(&self) -> &RegionColor {
        &self.color
    }
}

#[derive(Debug, Clone)]
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
    #[must_use]
    pub fn start_point(&self) -> usize {
        match self {
            Region::Vertical { y_start, .. } => *y_start,
            Region::Horizontal { x_start, .. } => *x_start,
        }
    }

    #[must_use]
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

    #[must_use]
    pub fn fixed_point(&self) -> usize {
        match self {
            Region::Vertical { x, .. } => *x,
            Region::Horizontal { y, .. } => *y,
        }
    }

    /// Get the position of the corresponding line spot
    ///
    /// The line spot is the point in the middle of the region.
    #[must_use]
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

    #[must_use]
    pub fn direction(&self) -> Direction {
        match self {
            Region::Vertical { .. } => Direction::Vertical,
            Region::Horizontal { .. } => Direction::Horizontal,
        }
    }

    #[must_use]
    pub fn length(&self) -> usize {
        self.end_point() - self.start_point()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Horizontal,
    Vertical,
}

fn get_horizontal_scan_lines<T: CameraLocation>(
    config: &ScanLinesConfig,
    field: &FieldColorApproximate,
    yuyv: &heimdall::YuyvImage,
    scan_grid: &ScanGrid<T>,
    field_boundary: &FieldBoundary,
) -> ScanLine {
    let mut regions = Vec::with_capacity(scan_grid.lines.len() * 4);
    let edge_threshold = config.min_edge_luminance_difference;

    let mut boundary_cache = Vec::new();
    if T::POSITION == CameraPosition::Top {
        boundary_cache = scan_grid
            .lines
            .iter()
            .map(|l| field_boundary.height_at_pixel(l.x as f32) as usize)
            .collect();
    }

    for &y in &scan_grid.y {
        let mut current: Option<ScanLineRegion> = None;

        for (line_idx, line) in scan_grid.lines.iter().enumerate() {
            if T::POSITION == CameraPosition::Top && y < boundary_cache[line_idx] {
                continue;
            }

            let x = line.x as usize;
            // SAFETY: we access x‑1, x, x+1; scan grid guarantees these are in‑bounds.
            let p = unsafe {
                avg_three(
                    yuyv.pixel_unchecked(x - 1, y),
                    yuyv.pixel_unchecked(x, y),
                    yuyv.pixel_unchecked(x + 1, y),
                )
            };

            if let Some(curr) = current.as_mut() {
                let diff = f32::from(u8::abs_diff(p.y, curr.approx_color.y));

                if diff >= edge_threshold {
                    let edge_x =
                        find_edge(yuyv, curr.region.end_point(), x, y, Direction::Horizontal);
                    curr.region.set_end_point(edge_x);

                    let finished = std::mem::replace(
                        curr,
                        ScanLineRegion {
                            region: Region::Horizontal {
                                y,
                                x_start: edge_x,
                                x_end: x,
                            },
                            approx_color: p,
                        },
                    );

                    regions.push(finished.classify(config, field));
                } else {
                    let len = x - curr.region.end_point();
                    curr.region.set_end_point(x);
                    curr.add_sample(p, len);
                }
            } else {
                current = Some(ScanLineRegion {
                    region: Region::Horizontal {
                        y,
                        x_start: x,
                        x_end: x,
                    },
                    approx_color: p,
                });
            }
        }

        if let Some(region) = current.take() {
            regions.push(region.classify(config, field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

fn get_vertical_scan_lines<T: CameraLocation>(
    config: &ScanLinesConfig,
    field: &FieldColorApproximate,
    yuyv: &heimdall::YuyvImage,
    scan_grid: &ScanGrid<T>,
    field_boundary: &FieldBoundary,
) -> ScanLine {
    let mut regions = Vec::with_capacity(scan_grid.lines.len() * 4);
    let edge_threshold = config.min_edge_luminance_difference;

    let mut boundary_cache = Vec::new();
    if T::POSITION == CameraPosition::Top {
        boundary_cache = scan_grid
            .lines
            .iter()
            .map(|l| field_boundary.height_at_pixel(l.x as f32) as usize)
            .collect();
    }

    let usable_y = &scan_grid.y[1..scan_grid.y.len() - 1];

    for (idx, line) in scan_grid.lines.iter().enumerate() {
        let mut current: Option<ScanLineRegion> = None;
        let x = line.x as usize;
        let boundary_y = if T::POSITION == CameraPosition::Top {
            boundary_cache[idx]
        } else {
            0
        };

        for &y in usable_y {
            if T::POSITION == CameraPosition::Top && y < boundary_y {
                continue;
            }

            // SAFETY: we access y‑1, y, y+1; usable_y skips the first & last rows.
            let p = unsafe {
                avg_three(
                    yuyv.pixel_unchecked(x, y - 1),
                    yuyv.pixel_unchecked(x, y),
                    yuyv.pixel_unchecked(x, y + 1),
                )
            };

            if let Some(curr) = current.as_mut() {
                let diff = f32::from(u8::abs_diff(p.y, curr.approx_color.y));

                if diff >= edge_threshold {
                    let edge_y =
                        find_edge(yuyv, curr.region.end_point(), y, x, Direction::Vertical);
                    curr.region.set_end_point(edge_y);

                    let finished = std::mem::replace(
                        curr,
                        ScanLineRegion {
                            region: Region::Vertical {
                                x,
                                y_start: edge_y,
                                y_end: y,
                            },
                            approx_color: p,
                        },
                    );

                    regions.push(finished.classify(config, field));
                } else {
                    let len = y - curr.region.end_point();
                    curr.region.set_end_point(y);
                    curr.add_sample(p, len);
                }
            } else {
                current = Some(ScanLineRegion {
                    region: Region::Vertical {
                        x,
                        y_start: y,
                        y_end: y,
                    },
                    approx_color: p,
                });
            }
        }

        if let Some(region) = current.take() {
            regions.push(region.classify(config, field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

fn get_scan_lines<T: CameraLocation>(
    config: &ScanLinesConfig,
    image: Image<T>,
    scan_grid: &ScanGrid<T>,
    field_boundary: &FieldBoundary,
) -> ScanLines<T> {
    let yuyv = image.yuyv_image();

    let field = FieldColorApproximate::new(yuyv);

    let horizontal = get_horizontal_scan_lines(config, &field, yuyv, scan_grid, field_boundary);
    let vertical = get_vertical_scan_lines(config, &field, yuyv, scan_grid, field_boundary);

    ScanLines::new(image, horizontal, vertical)
}

pub fn init_scan_lines<T: CameraLocation>(mut commands: Commands, image: Res<Image<T>>) {
    let scan_lines = ScanLines::new(
        image.clone(),
        ScanLine { raw: vec![] },
        ScanLine { raw: vec![] },
    );

    commands.insert_resource(scan_lines);
}

pub fn update_scan_lines<T: CameraLocation>(
    config: Res<ScanLinesConfig>,
    mut scan_lines: ResMut<ScanLines<T>>,
    image: Res<Image<T>>,
    scan_grid: Res<ScanGrid<T>>,
    field_boundary: Res<FieldBoundary>,
) {
    *scan_lines = get_scan_lines(&config, image.clone(), &scan_grid, &field_boundary);
}

/// Find the strongest luminance edge between `start` and `end` (inclusive).
#[inline]
fn find_edge(
    yuyv: &heimdall::YuyvImage,
    start: usize,
    end: usize,
    fixed: usize,
    direction: Direction,
) -> usize {
    let mut best_pos = start;
    let mut best_diff = 0u8;

    match direction {
        Direction::Horizontal => {
            let mut prev_y = unsafe { yuyv.pixel_unchecked(start, fixed).y };
            for pos in (start + 1)..=end {
                let y = unsafe { yuyv.pixel_unchecked(pos, fixed).y };
                let diff = prev_y.abs_diff(y);
                if diff > best_diff {
                    best_diff = diff;
                    best_pos = pos - 1;
                }
                prev_y = y;
            }
        }
        Direction::Vertical => {
            let mut prev_y = unsafe { yuyv.pixel_unchecked(fixed, start).y };
            for pos in (start + 1)..=end {
                let y = unsafe { yuyv.pixel_unchecked(fixed, pos).y };
                let diff = prev_y.abs_diff(y);
                if diff > best_diff {
                    best_diff = diff;
                    best_pos = pos - 1;
                }
                prev_y = y;
            }
        }
    }

    best_pos
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
    #[must_use]
    pub fn classify_yuv_pixel(
        config: &ScanLinesConfig,
        _field: &FieldColorApproximate,
        pixel: YuvPixel,
    ) -> Self {
        let yhs = pixel.to_yhs2();
        let (r, g, b) = pixel.to_rgb();

        let color_sum = r + g + b;
        let g_chromaticity = g / color_sum;
        let green_threshold = config.green_chromaticity_threshold;

        if Self::is_black(config, yhs) && g_chromaticity <= green_threshold {
            // We mark black spots as white regions for ball detection
            return RegionColor::WhiteOrBlack;
        }

        // use chromaticity to find green pixels
        if g_chromaticity > green_threshold {
            return RegionColor::Green;
        }

        if Self::is_white(config, yhs) {
            return RegionColor::WhiteOrBlack;
        }

        RegionColor::Unknown
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

#[allow(unused)]
fn visualize_scan_lines<T: CameraLocation>(dbg: DebugContext, scan_lines: Res<ScanLines<T>>) {
    visualize_single_scan_line::<T>(&dbg, scan_lines.horizontal(), scan_lines.image().cycle());
    visualize_single_scan_line::<T>(&dbg, scan_lines.vertical(), scan_lines.image().cycle());

    visualize_scan_line_spots::<T>(
        &dbg,
        scan_lines.horizontal(),
        scan_lines.image().cycle(),
        rerun::Color::from_rgb(255, 0, 0),
    );
    visualize_scan_line_spots::<T>(
        &dbg,
        scan_lines.vertical(),
        scan_lines.image().cycle(),
        rerun::Color::from_rgb(0, 0, 255),
    );
}

#[allow(unused)]
fn visualize_single_scan_line<T: CameraLocation>(
    dbg: &DebugContext,
    scan_line: &ScanLine,
    cycle: Cycle,
) {
    let scan_line = &scan_line.raw;
    if scan_line.is_empty() {
        return;
    }

    let direction = scan_line[0].line.region.direction();
    let direction_str = match direction {
        Direction::Horizontal => "horizontal",
        Direction::Vertical => "vertical",
    };

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

        colors.push(rerun::Color::from_rgb(r, g, b));

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
        classifications.push(rerun::Color::from_rgb(r, g, b));
    }

    dbg.log_with_cycle(
        T::make_entity_image_path(format!("scan_lines/approximates/{direction_str}")),
        cycle,
        &rerun::LineStrips2D::new(lines.clone()).with_colors(colors),
    );

    dbg.log_with_cycle(
        T::make_entity_image_path(format!("scan_lines/classifications/{direction_str}")),
        cycle,
        &rerun::LineStrips2D::new(lines).with_colors(classifications),
    );
}

fn visualize_scan_line_spots<T: CameraLocation>(
    dbg: &DebugContext,
    scan_line: &ScanLine,
    cycle: Cycle,
    color: rerun::Color,
) {
    let regions = &scan_line.raw;

    if regions.is_empty() {
        return;
    }

    let direction = regions[0].line.region.direction();
    let direction_str = match direction {
        Direction::Horizontal => "horizontal",
        Direction::Vertical => "vertical",
    };

    let line_spots = scan_line
        .line_spots()
        .map(|s| (s.x, s.y))
        .collect::<Vec<_>>();

    let colors = vec![color; line_spots.len()];

    dbg.log_with_cycle(
        T::make_entity_image_path(format!("scan_lines/spots/{direction_str}")),
        cycle,
        &rerun::Points2D::new(line_spots).with_colors(colors),
    );
}
