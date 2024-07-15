use crate::{core::debug::DebugContext, nao::Cycle, prelude::*};

use super::{
    camera::{BottomImage, Image, TopImage},
    color,
    field_boundary::FieldBoundary,
    scan_grid::{FieldColorApproximate, ScanGrid, ScanGrids},
};

use heimdall::{YuvPixel, YuyvImage};
use nalgebra::Point2;
use nidhogg::types::{color::u8, RgbU8};

const MIN_EDGE_LUMINANCE_DIFFERENCE: f32 = 13.0;

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// TODO: resources
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(scan_lines_system.after(super::scan_grid::update_scan_grid))
            .init_resource::<TopScanLines>()?
            .init_resource::<BottomScanLines>()
    }
}

#[derive(Debug, Default)]
pub struct ScanLines {
    pub horizontal: ScanLine,
    pub vertical: ScanLine,
}

#[derive(Debug, Default)]
pub struct ScanLine {
    raw: Vec<ClassifiedScanLineRegion>,
}

impl ScanLine {
    pub fn new(raw: Vec<ClassifiedScanLineRegion>) -> Self {
        Self { raw }
    }

    pub fn line_spots(&self) -> impl Iterator<Item = Point2<f32>> + '_ {
        self.raw
            .iter()
            .filter(|r| matches!(r.color, RegionColor::White))
            .map(|r| r.line.region.line_spot())
    }
}

#[derive(Debug, derive_more::Deref, derive_more::DerefMut, Default)]
pub struct TopScanLines(ScanLines);

impl TopScanLines {
    pub fn new(horizontal: ScanLine, vertical: ScanLine) -> Self {
        Self(ScanLines {
            horizontal,
            vertical,
        })
    }
}

#[derive(Debug, derive_more::Deref, derive_more::DerefMut, Default)]
pub struct BottomScanLines(ScanLines);

impl BottomScanLines {
    pub fn new(horizontal: ScanLine, vertical: ScanLine) -> Self {
        Self(ScanLines {
            horizontal,
            vertical,
        })
    }
}

#[derive(Debug)]
struct ScanLineRegion {
    region: Region,
    approx_color: YuvPixel,
}

impl ScanLineRegion {
    pub fn add_sample(&mut self, sample: YuvPixel, weight: usize) {
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

    pub fn classify(self, field: &FieldColorApproximate) -> ClassifiedScanLineRegion {
        let color = RegionColor::classify_yuv_pixel(field, self.approx_color.clone());

        ClassifiedScanLineRegion { line: self, color }
    }
}

#[derive(Debug)]
pub struct ClassifiedScanLineRegion {
    line: ScanLineRegion,
    color: RegionColor,
}

impl ClassifiedScanLineRegion {
    /// Merges adjacent regions with the same color.
    pub fn simplify(regions: Vec<Self>) -> Vec<Self> {
        let mut new_regions = Vec::new();

        let mut current = None;
        for region in regions {
            let Some(mut curr) = current.take() else {
                current = Some(region);
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

                current = Some(curr);
            } else {
                new_regions.push(curr);
                current = Some(region);
            }
        }

        if let Some(curr) = current.take() {
            new_regions.push(curr);
        }

        new_regions
    }
}

#[derive(Debug)]
enum Region {
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

    pub fn set_end_point(&mut self, end_point: usize) {
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
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    camera: CameraType,
) -> ScanLine {
    let mut regions = Vec::new();

    for y in &scan_grid.y {
        let mut curr_region = None;
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

            let Some(curr_region) = &mut curr_region else {
                // first region of this y coordinate
                curr_region = Some(ScanLineRegion {
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

            if lum_diff > MIN_EDGE_LUMINANCE_DIFFERENCE {
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
                regions.push(new_region.classify(field));

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

        if let Some(curr_region) = curr_region.take() {
            regions.push(curr_region.classify(field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

fn get_vertical_scan_lines(
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    camera: CameraType,
) -> ScanLine {
    let mut regions = Vec::new();

    for line in &scan_grid.lines {
        let mut curr_region = None;

        for y in &scan_grid.y {
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

            let Some(curr_region) = &mut curr_region else {
                // first region of this y coordinate
                curr_region = Some(ScanLineRegion {
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

            if lum_diff > MIN_EDGE_LUMINANCE_DIFFERENCE {
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
                regions.push(new_region.classify(field));

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

        if let Some(curr_region) = curr_region.take() {
            regions.push(curr_region.classify(field));
        }
    }

    ScanLine::new(ClassifiedScanLineRegion::simplify(regions))
}

#[system]
fn scan_lines_system(
    (top_image, bottom_image): (&TopImage, &BottomImage),
    scan_grid: &ScanGrids,
    field_boundary: &FieldBoundary,
    (top_scan_lines, bottom_scan_lines): (&mut TopScanLines, &mut BottomScanLines),
    curr_cycle: &Cycle,
    dbg: &DebugContext,
) -> Result<()> {
    let top_grid = &scan_grid.top;
    let bottom_grid = &scan_grid.bottom;

    update_scan_lines(
        top_image,
        top_grid,
        field_boundary,
        top_scan_lines,
        curr_cycle,
        dbg,
        CameraType::Top,
    )?;

    update_scan_lines(
        bottom_image,
        bottom_grid,
        field_boundary,
        bottom_scan_lines,
        curr_cycle,
        dbg,
        CameraType::Bottom,
    )?;

    Ok(())
}

fn update_scan_lines(
    image: &Image,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    scan_lines: &mut ScanLines,
    curr_cycle: &Cycle,
    dbg: &DebugContext,
    camera: CameraType,
) -> Result<()> {
    if !scan_grid.image.is_from_cycle(curr_cycle) || scan_grid.lines.is_empty() {
        return Ok(());
    }

    let yuyv = scan_grid.image.yuyv_image();

    let field = FieldColorApproximate::new(yuyv);

    let scan_lines_h = get_horizontal_scan_lines(&field, yuyv, scan_grid, field_boundary, camera);
    let scan_lines_v = get_vertical_scan_lines(&field, yuyv, scan_grid, field_boundary, camera);

    debug_scan_lines(&scan_lines_h, dbg, image, camera)?;
    debug_scan_lines(&scan_lines_v, dbg, image, camera)?;

    debug_scan_line_spots(&scan_lines_h, dbg, image, u8::RED, camera)?;
    debug_scan_line_spots(&scan_lines_v, dbg, image, u8::BLUE, camera)?;

    scan_lines.horizontal = scan_lines_h;
    scan_lines.vertical = scan_lines_v;

    Ok(())
}

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
    White,
    Black,
    Green,
    Unknown,
}

impl RegionColor {
    // TODO: use our field color approximate
    pub fn classify_yuv_pixel(_field: &FieldColorApproximate, pixel: YuvPixel) -> Self {
        let yhs = pixel.to_yhs2();

        if Self::is_green(yhs) {
            return RegionColor::Green;
        }

        if Self::is_white(yhs) {
            return RegionColor::White;
        }

        if Self::is_black(yhs) {
            return RegionColor::Black;
        }

        RegionColor::Unknown
    }

    fn is_green((y, h, s): (f32, f32, f32)) -> bool {
        const MAX_FIELD_LUMINANCE: f32 = 200.0;
        const MIN_FIELD_SATURATION: f32 = 40.0;
        // TODO: our hues are broken methinks
        // const MIN_FIELD_HUE: f32 = 120.0;
        // const MAX_FIELD_HUE: f32 = 200.0;
        const MIN_FIELD_HUE: f32 = 0.0;
        const MAX_FIELD_HUE: f32 = 80.0;

        y <= MAX_FIELD_LUMINANCE
            && s >= MIN_FIELD_SATURATION
            && (MIN_FIELD_HUE..MAX_FIELD_HUE).contains(&h)
    }

    fn is_white((y, _h, s): (f32, f32, f32)) -> bool {
        // const MIN_WHITE_TO_FIELD_LUMINANCE_DIFFERENCE: f32 = 15.0;
        // const MIN_WHITE_TO_FIELD_SATURATION_DIFFERENCE: f32 = 15.0;
        const MAX_WHITE_SATURATION: f32 = 100.0;
        const MIN_WHITE_LUMINANCE: f32 = 90.0;

        y > MIN_WHITE_LUMINANCE && s < MAX_WHITE_SATURATION
    }

    fn is_black((y, _h, _s): (f32, f32, f32)) -> bool {
        const MAX_BLACK_LUMINANCE: f32 = 50.0;
        // const MAX_BLACK_SATURATION: f32 = 50.0;

        y < MAX_BLACK_LUMINANCE // && s < MAX_BLACK_SATURATION
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum CameraType {
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
            RegionColor::White => (255, 255, 255),
            RegionColor::Black => (0, 0, 0),
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
