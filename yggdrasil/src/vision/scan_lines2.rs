use crate::{core::debug::DebugContext, nao::Cycle, prelude::*};

use super::{
    camera::TopImage,
    field_boundary::FieldBoundary,
    scan_grid::{FieldColorApproximate, ScanGrid},
};

use heimdall::{YuvPixel, YuyvImage};
use nalgebra::Point2;
use nidhogg::types::RgbU8;

const MIN_EDGE_LUMINANCE_DIFFERENCE: f32 = 13.0;

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// TODO: resources
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(scan_lines_system.after(super::scan_grid::update_scan_grid)))
        // .add_startup_system(init_buffers)
    }
}

pub struct ScanLines;

fn yuy2_to_rgb((y, u, v): (u8, u8, u8)) -> (u8, u8, u8) {
    let (y, u, v) = (y as f32, u as f32, v as f32);

    // rescale YUV values
    let y = (y - 16.0) / 219.0;
    let u = (u - 128.0) / 224.0;
    let v = (v - 128.0) / 224.0;

    // BT.601 (aka. SDTV, aka. Rec.601). wiki: https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.601_conversion
    let r = y + 1.402 * v;
    let g = y - 0.344 * u - 0.714 * v;
    let b = y + 1.772 * u;

    // BT.709 (aka. HDTV, aka. Rec.709). wiki: https://en.wikipedia.org/wiki/YCbCr#ITU-R_BT.709_conversion
    // let r = y + 1.575 * v;
    // let g = y - 0.187 * u - 0.468 * v;
    // let b = y + 1.856 * u;

    (
        (255.0 * r).clamp(0.0, 255.0) as u8,
        (255.0 * g).clamp(0.0, 255.0) as u8,
        (255.0 * b).clamp(0.0, 255.0) as u8,
    )
}

#[derive(Debug)]
struct ScanLineRegion {
    region: Region,
    approx_color: YuvPixel,
}

impl ScanLineRegion {
    pub fn classify(self, field: &FieldColorApproximate) -> ClassifiedScanLineRegion {
        let color = RegionColor::classify_yuv_pixel(field, self.approx_color.clone());

        ClassifiedScanLineRegion { line: self, color }
    }
}

#[derive(Debug)]
struct ClassifiedScanLineRegion {
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
                curr.line.approx_color = YuvPixel::average(&[
                    curr.line.approx_color.clone(),
                    region.line.approx_color.clone(),
                ]);
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
) -> Vec<ClassifiedScanLineRegion> {
    let mut regions = Vec::new();

    for y in &scan_grid.y {
        let mut curr_region = None;

        for line in &scan_grid.lines {
            let x = line.x as usize;
            let y = *y;

            // skip lines above the field boundary
            let boundary = field_boundary.height_at_pixel(x as f32) as usize;
            if y < boundary {
                continue;
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
                curr_region.region.set_end_point(x);
                curr_region.approx_color =
                    YuvPixel::average(&[curr_region.approx_color.clone(), pixel]);
            }
        }

        if let Some(curr_region) = curr_region.take() {
            regions.push(curr_region.classify(field));
        }
    }

    ClassifiedScanLineRegion::simplify(regions)
}

fn get_vertical_scan_lines(
    field: &FieldColorApproximate,
    yuyv: &YuyvImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
) -> Vec<ClassifiedScanLineRegion> {
    let mut regions = Vec::new();

    for line in &scan_grid.lines {
        let mut curr_region = None;
        for y in &scan_grid.y {
            let x = line.x as usize;
            let y = *y;

            // skip lines above the field boundary
            let boundary = field_boundary.height_at_pixel(line.x as f32) as usize;
            if y < boundary {
                continue;
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
                curr_region.region.set_end_point(y);
                curr_region.approx_color =
                    YuvPixel::average(&[curr_region.approx_color.clone(), pixel]);
            }
        }

        if let Some(curr_region) = curr_region.take() {
            regions.push(curr_region.classify(field));
        }
    }

    ClassifiedScanLineRegion::simplify(regions)
}

#[system]
fn scan_lines_system(
    image: &TopImage,
    scan_grid: &ScanGrid,
    field_boundary: &FieldBoundary,
    // scan_lines: &mut ScanLines,
    curr_cycle: &Cycle,
    dbg: &DebugContext,
) -> Result<()> {
    if !scan_grid.image.is_from_cycle(curr_cycle) || scan_grid.lines.is_empty() {
        return Ok(());
    }

    let yuyv = scan_grid.image.yuyv_image();

    let field = FieldColorApproximate::new(yuyv);

    let regions_h = get_horizontal_scan_lines(&field, yuyv, scan_grid, field_boundary);
    let regions_v = get_vertical_scan_lines(&field, yuyv, scan_grid, field_boundary);

    debug_scan_lines(&regions_h, dbg, image)?;
    debug_scan_lines(&regions_v, dbg, image)?;

    let line_spots_h = regions_h
        .iter()
        .filter(|r| matches!(r.color, RegionColor::White))
        .map(|r| r.line.region.line_spot())
        .map(|s| (s.x, s.y))
        .collect::<Vec<_>>();

    let line_spots_v = regions_v
        .iter()
        .filter(|r| matches!(r.color, RegionColor::White))
        .map(|r| r.line.region.line_spot())
        .map(|s| (s.x, s.y))
        .collect::<Vec<_>>();

    let colors_h = vec![RgbU8::new(255, 0, 0); line_spots_h.len()];
    let colors_v = vec![RgbU8::new(0, 0, 255); line_spots_v.len()];

    dbg.log_points2d_for_image_with_colors(
        "top_camera/image/horizontal_line_spots",
        &line_spots_h,
        image,
        &colors_h,
    )?;

    dbg.log_points2d_for_image_with_colors(
        "top_camera/image/vertical_line_spots",
        &line_spots_v,
        image,
        &colors_v,
    )?;

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
                Direction::Horizontal => {
                    (unsafe { yuyv.pixel_unchecked(pos_next, fixed) }, unsafe {
                        yuyv.pixel_unchecked(pos_next + 1, fixed)
                    })
                }
                Direction::Vertical => (unsafe { yuyv.pixel_unchecked(fixed, pos_next) }, unsafe {
                    yuyv.pixel_unchecked(fixed, pos_next + 1)
                }),
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
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RegionColor {
    White,
    Black,
    Green,
    Unknown,
}

impl RegionColor {
    pub fn yuv_to_yhs(pixel: YuvPixel) -> (f32, f32, f32) {
        let YuvPixel { y, u, v } = pixel;
        let (y, u, v) = (y as i32, u as i32, v as i32);

        let v_normed = v - 128;
        let u_normed = u - 128;

        let h = fast_math::atan2(v_normed as f32, u_normed as f32)
            * std::f32::consts::FRAC_1_PI
            * 127.0
            + 127.0;
        let s = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y as f32;

        (y as f32, h, s)
    }

    pub fn classify_yuv_pixel(_field: &FieldColorApproximate, pixel: YuvPixel) -> Self {
        // TODO: use field color approximate somehow

        let yhs = Self::yuv_to_yhs(pixel);

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

fn debug_scan_lines(
    regions: &[ClassifiedScanLineRegion],
    dbg: &DebugContext,
    image: &TopImage,
) -> Result<()> {
    let direction = regions[0].line.region.direction();

    let region_len = regions.len();

    let mut lines = Vec::with_capacity(region_len);
    let mut colors = Vec::with_capacity(region_len);
    let mut classifications = Vec::with_capacity(region_len);

    for line in regions {
        let (r, g, b) = yuy2_to_rgb((
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

    dbg.log_lines2d_for_image_with_colors(
        format!("top_camera/image/{direction_str}_scan_lines"),
        &lines,
        image,
        &colors,
    )?;

    dbg.log_lines2d_for_image_with_colors(
        format!("top_camera/image/{direction_str}_scan_lines_classifications"),
        &lines,
        image,
        &classifications,
    )?;

    Ok(())
}
