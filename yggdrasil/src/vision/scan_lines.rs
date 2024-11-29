use std::{ops::Deref, sync::Arc};

use crate::{
    core::debug::{debug_system::DebugAppExt, DebugContext},
    nao::Cycle,
    prelude::*,
};

use super::{
    camera::{init_camera, Image},
    color,
    field_boundary::FieldBoundary,
    scan_grid::{FieldColorApproximate, ScanGrid},
};
use bevy::prelude::*;

use heimdall::{Bottom, CameraLocation, CameraPosition, Top, YuvPixel, YuyvImage};
use nalgebra::Point2;
use serde::{Deserialize, Serialize};

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
}

impl Config for ScanLinesConfig {
    const PATH: &'static str = "scan_lines.toml";
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
                    visualize_scan_lines::<Top>.run_if(resource_exists::<ScanLines<Top>>),
                    visualize_scan_lines::<Bottom>.run_if(resource_exists::<ScanLines<Bottom>>),
                ),
                "Visualize scan lines",
            );
    }
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
        let color = RegionColor::classify_yuv_pixel(config, field, self.approx_color.clone());

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

    #[must_use]
    pub fn scan_line_region(&self) -> &ScanLineRegion {
        &self.line
    }

    #[must_use]
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
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid<T>,
    field_boundary: &FieldBoundary,
) -> ScanLine {
    let mut regions = Vec::new();

    for y in &scan_grid.y {
        let mut current_region = None;
        for line in &scan_grid.lines {
            let x = line.x as usize;
            let y = *y;

            if T::POSITION == CameraPosition::Top {
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

            let lum_diff = (f32::from(pixel.y) - f32::from(curr_region.approx_color.y)).abs();

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

fn get_vertical_scan_lines<T: CameraLocation>(
    config: &ScanLinesConfig,
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid<T>,
    field_boundary: &FieldBoundary,
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

            if T::POSITION == CameraPosition::Top {
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

            let lum_diff = (f32::from(pixel.y) - f32::from(curr_region.approx_color.y)).abs();

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

            let lum = f32::from(pixel.y);
            let lum_next = f32::from(pixel_next.y);

            let next_diff = (lum_next - lum).abs();

            if next_diff > diff_edge {
                (pos_next, next_diff)
            } else {
                (pos_edge, diff_edge)
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
    #[must_use]
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
        T::make_entity_path(format!("scan_lines/approximates/{direction_str}")),
        cycle,
        &rerun::LineStrips2D::new(lines.clone()).with_colors(colors),
    );

    dbg.log_with_cycle(
        T::make_entity_path(format!("scan_lines/classifications/{direction_str}")),
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
        T::make_entity_path(format!("scan_lines/spots/{direction_str}")),
        cycle,
        &rerun::Points2D::new(line_spots).with_colors(colors),
    );
}
