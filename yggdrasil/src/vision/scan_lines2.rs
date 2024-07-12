use crate::{core::debug::DebugContext, nao::Cycle, prelude::*};

use super::{
    camera::TopImage,
    field_boundary::{self, FieldBoundary},
    scan_grid::{FieldColorApproximate, ScanGrid},
    scan_lines,
};

use heimdall::{YuvPixel, YuyvImage};
use nidhogg::types::{color, RgbU8};

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`TopScanGrid`]
/// - [`BottomScanGrid`]
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(scan_lines_system.after(super::scan_grid::update_scan_grid)))
        // .add_startup_system(init_buffers)
    }
}

pub struct ScanLines;

fn yuy2_to_rgb((y, u, v): (u8, u8, u8)) -> (u8, u8, u8) {
    fn clamp(value: i32) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        return value.clamp(0, 255) as u8;
    }

    let y = i32::from(y) - 16;
    let u = i32::from(u) - 128;
    let v = i32::from(v) - 128;

    let red = (298 * y + 409 * v + 128) >> 8;
    let green = (298 * y - 100 * u - 208 * v + 128) >> 8;
    let blue = (298 * y + 516 * u + 128) >> 8;

    (clamp(red), clamp(green), clamp(blue))
}

#[derive(Debug)]
struct HorizontalRegion {
    y: usize,
    x_start: usize,
    x_end: usize,
    pixel: YuvPixel,
}

#[derive(Debug)]
struct HorizontalColorRegion {
    y: usize,
    x_start: usize,
    x_end: usize,
    color: PixelColor,
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

    let mut regions = Vec::new();

    for y in &scan_grid.y {
        let mut curr_region = None;

        for line in &scan_grid.lines {
            let boundary = field_boundary.height_at_pixel(line.x as f32) as usize;
            if *y < boundary {
                continue;
            }

            let pixels = unsafe {
                [
                    yuyv.pixel_unchecked(line.x as usize - 1, *y),
                    yuyv.pixel_unchecked(line.x as usize, *y),
                    yuyv.pixel_unchecked(line.x as usize + 1, *y),
                ]
            };

            let pixel = YuvPixel::average(&pixels);

            let Some(curr_region) = &mut curr_region else {
                curr_region = Some(HorizontalRegion {
                    y: *y,
                    x_start: line.x as usize,
                    x_end: line.x as usize,
                    pixel: pixel,
                });
                continue;
            };

            let lum_diff = (pixel.y as f32 - curr_region.pixel.y as f32).abs();

            if lum_diff > 13.0 {
                let x_edge = find_edge(
                    yuyv,
                    curr_region.x_end,
                    line.x as usize,
                    *y,
                    // EdgeType::Rising,
                );

                curr_region.x_end = x_edge;

                let mut new_region = HorizontalRegion {
                    y: *y,
                    x_start: x_edge,
                    x_end: line.x as usize,
                    pixel: pixel,
                };

                // put new region in place of curr_region
                std::mem::swap(curr_region, &mut new_region);
                // and push the old region to the vec
                regions.push(new_region);

                continue;
            } else {
                curr_region.x_end = line.x as usize;
                curr_region.pixel = YuvPixel::average(&[curr_region.pixel.clone(), pixel]);
            }
        }

        if let Some(mut curr_region) = curr_region.take() {
            curr_region.x_end = yuyv.width();
            regions.push(curr_region);
        }
    }

    let regions: Vec<_> = regions
        .into_iter()
        .map(|r| {
            let region_color = PixelColor::classify_yuv_pixel_new(&field, r.pixel);

            HorizontalColorRegion {
                y: r.y,
                x_start: r.x_start,
                x_end: r.x_end,
                color: region_color,
            }
        })
        .collect();

    debug_scan_line_regions(&regions, dbg, image)?;

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeType {
    Rising,
    Falling,
}

fn find_edge(
    yuyv: &YuyvImage,
    x_start: usize,
    x_end: usize,
    y: usize,
    // edge_type: EdgeType,
) -> usize {
    let (x_edge, _value_edge) =
        (x_start..=x_end).fold((x_start, 0.0), |(x_edge, diff_edge), x_next| {
            let pixel = unsafe { yuyv.pixel_unchecked(x_next, y) };
            let pixel_next = unsafe { yuyv.pixel_unchecked(x_next + 1, y) };

            let lum = pixel.y as f32;
            let lum_next = pixel_next.y as f32;

            // let next_diff = lum_next - lum;
            let next_diff = (lum_next - lum).abs();

            // match (edge_type, next_diff > diff_edge) {
            //     (EdgeType::Rising, true) => (x_next, next_diff),
            //     (EdgeType::Rising, false) => (x_edge, diff_edge),
            //     (EdgeType::Falling, true) => (x_edge, diff_edge),
            //     (EdgeType::Falling, false) => (x_next, next_diff),
            // }
            match next_diff > diff_edge {
                true => (x_next, next_diff),
                false => (x_edge, diff_edge),
            }
        });

    x_edge
}

fn debug_scan_line_regions(
    regions: &[HorizontalColorRegion],
    dbg: &DebugContext,
    image: &TopImage,
) -> Result<()> {
    let mut lines = Vec::new();
    let mut colors = Vec::new();
    for line in regions {
        // let (r, g, b) = yuy2_to_rgb((line.pixel.y, line.pixel.u, line.pixel.v));

        let (r, g, b) = match line.color {
            PixelColor::White => (255, 255, 255),
            PixelColor::Black => (0, 0, 0),
            PixelColor::Green => (0, 255, 0),
            PixelColor::Unknown => (128, 128, 128),
        };

        lines.push([
            (line.x_start as f32, line.y as f32),
            (line.x_end as f32, line.y as f32),
        ]);

        colors.push(RgbU8::new(r, g, b));
    }

    dbg.log_lines2d_for_image_with_colors(
        "top_camera/image/scan_line_region_lines",
        &lines,
        image,
        &colors,
    )?;

    Ok(())
}

fn debug_scan_grid_color_thingy(
    scan_line_points: &[(usize, usize, PixelColor)],
    dbg: &DebugContext,
    image: &TopImage,
) -> Result<()> {
    let green_points = scan_line_points
        .iter()
        .filter(|(_, _, color)| *color == PixelColor::Green)
        .map(|(x, y, _)| (*x as f32, *y as f32))
        .collect::<Vec<_>>();

    let white_points = scan_line_points
        .iter()
        .filter(|(_, _, color)| *color == PixelColor::White)
        .map(|(x, y, _)| (*x as f32, *y as f32))
        .collect::<Vec<_>>();

    let black_points = scan_line_points
        .iter()
        .filter(|(_, _, color)| *color == PixelColor::Black)
        .map(|(x, y, _)| (*x as f32, *y as f32))
        .collect::<Vec<_>>();

    let unknown_points = scan_line_points
        .iter()
        .filter(|(_, _, color)| *color == PixelColor::Unknown)
        .map(|(x, y, _)| (*x as f32, *y as f32))
        .collect::<Vec<_>>();

    dbg.log_points2d_for_image(
        "top_camera/image/scan_grid_points/green",
        &green_points,
        image,
        color::u8::GREEN,
    )?;

    dbg.log_points2d_for_image(
        "top_camera/image/scan_grid_points/white",
        &white_points,
        image,
        color::u8::WHITE,
    )?;

    dbg.log_points2d_for_image(
        "top_camera/image/scan_grid_points/black",
        &black_points,
        image,
        color::Rgb::new(0, 0, 0),
    )?;

    dbg.log_points2d_for_image(
        "top_camera/image/scan_grid_points/unknown",
        &unknown_points,
        image,
        color::u8::GRAY,
    )?;

    Ok(())
}

/// The classified color of a scan-line pixel.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PixelColor {
    White,
    Black,
    Green,
    Unknown,
}

impl PixelColor {
    #[deprecated]
    pub fn yuv_to_yhs(y1: u8, u: u8, v: u8) -> (f32, f32, f32) {
        let y1 = y1 as i32;
        let u = u as i32;
        let v = v as i32;

        let v_normed = v - 128;
        let u_normed = u - 128;

        let y = y1;
        let h =
            fast_math::atan2(v_normed as f32, u_normed as f32) * std::f32::consts::FRAC_1_PI * 127.
                + 127.;
        let s = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y as f32;

        (y as f32, h, s)
    }

    pub fn yuv_to_yhs_new(pixel: YuvPixel) -> (f32, f32, f32) {
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

    #[deprecated]
    pub fn classify_yuv_pixel(field: &FieldColorApproximate, y: u8, u: u8, v: u8) -> Self {
        let (y, h, s) = Self::yuv_to_yhs(y, u, v);

        let (lum_mean, lum_std) = field.luminance;

        let lum_diff = y - lum_mean;

        // dbg!(lum_diff, y, lum_mean, lum_std);

        if Self::is_green(field, y, h, s, 2.0) {
            return PixelColor::Green;
        }

        if lum_diff > 1.0 * lum_std {
            return PixelColor::White;
        } else if lum_diff < 2.0 * lum_std {
            return PixelColor::Black;
        }

        PixelColor::Unknown
    }

    pub fn classify_yuv_pixel_new(_field: &FieldColorApproximate, pixel: YuvPixel) -> Self {
        let yhs = Self::yuv_to_yhs_new(pixel);

        if Self::is_green_new(yhs) {
            return PixelColor::Green;
        }

        if Self::is_white_new(yhs) {
            return PixelColor::White;
        }

        if Self::is_black_new(yhs) {
            return PixelColor::Black;
        }

        // let (lum_mean, lum_std) = field.luminance;

        PixelColor::Unknown
    }

    #[deprecated]
    fn is_green(field: &FieldColorApproximate, y: f32, h: f32, s: f32, leniency: f32) -> bool {
        let lum_diff = (field.luminance.0 - y).abs();
        let hue_diff = (field.hue.0 - h).abs();
        let sat_diff = (field.saturation.0 - s).abs();

        lum_diff < field.luminance.1 * leniency
            && hue_diff < field.hue.1 * leniency
            && sat_diff < field.saturation.1 * leniency
    }

    fn is_green_new((y, h, s): (f32, f32, f32)) -> bool {
        const MAX_FIELD_LUMINANCE: f32 = 200.0;
        const MIN_FIELD_SATURATION: f32 = 30.0;
        // TODO: our hues are broken methinks
        // const MIN_FIELD_HUE: f32 = 120.0;
        // const MAX_FIELD_HUE: f32 = 200.0;
        const MIN_FIELD_HUE: f32 = 0.0;
        const MAX_FIELD_HUE: f32 = 80.0;

        y <= MAX_FIELD_LUMINANCE
            && s >= MIN_FIELD_SATURATION
            && (MIN_FIELD_HUE..MAX_FIELD_HUE).contains(&h)
    }

    fn is_white_new((y, _h, s): (f32, f32, f32)) -> bool {
        // const MIN_WHITE_TO_FIELD_LUMINANCE_DIFFERENCE: f32 = 15.0;
        // const MIN_WHITE_TO_FIELD_SATURATION_DIFFERENCE: f32 = 15.0;
        const MAX_WHITE_SATURATION: f32 = 100.0;
        const MIN_WHITE_LUMINANCE: f32 = 90.0;

        y > MIN_WHITE_LUMINANCE && s < MAX_WHITE_SATURATION
    }

    fn is_black_new((y, _h, _s): (f32, f32, f32)) -> bool {
        const MAX_BLACK_LUMINANCE: f32 = 50.0;
        // const MAX_BLACK_SATURATION: f32 = 50.0;

        y < MAX_BLACK_LUMINANCE
    }
}
