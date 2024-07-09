use crate::{core::debug::DebugContext, nao::Cycle, prelude::*};

use super::{
    camera::TopImage,
    field_boundary::FieldBoundary,
    scan_grid::{FieldColorApproximate, ScanGrid},
};

use nidhogg::types::color;

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

fn rgb_from_hsv((h, s, v): (f32, f32, f32)) -> [f32; 3] {
    #![allow(clippy::many_single_char_names)]
    let h = (h.fract() + 1.0).fract(); // wrap
    let s = s.clamp(0.0, 1.0);

    let f = h * 6.0 - (h * 6.0).floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);

    match (h * 6.0).floor() as i32 % 6 {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        5 => [v, p, q],
        _ => unreachable!(),
    }
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

    // let mut imaaaaa = image::ImageBuffer::new(640, 480);
    // println!("yuyv y: {:?}", yuyv.row_iter().count());
    // println!("yuyv x: {:?}", yuyv.row_iter().next().unwrap().count());
    // for (y_, row) in yuyv.row_iter().enumerate() {
    //     for (x, pixel) in row.enumerate() {
    //         let (y, h, s) = pixel.to_yhs2();
    //         // println!("yhs={}, {}, {} ", y, h, s);
    //         let [r, g, b] = rgb_from_hsv((h, s, y));
    //         // println!("{}, {}, {} ", r, g, b);
    //         // let (r, g, b) = (r * 255.0, g * 255.0, b * 255.0);
    //         // println!("{}, {}, {} ", r, g, b);
    //         let (r, g, b) = (r as u8, g as u8, b as u8);
    //         // println!("{}, {}, {} ", r, g, b);
    //         // println!("===============");
    //         imaaaaa.put_pixel(x as u32, y_ as u32, image::Rgb([r, g, b]));
    //     }
    // }

    // dbg.log_image_rgb("image/yuv_converted", imaaaaa, &scan_grid.image.cycle)?;

    let field = FieldColorApproximate::new(yuyv);

    // println!("field: {:?}, cycle: {}", field, curr_cycle.0);

    // TODO: i think this is always the same
    // TODO: should probably simplify the whole scan grid algorithm

    let mut scan_line_points = Vec::new();

    for line in &scan_grid.lines {
        for y in scan_grid.y.iter().filter(|y| **y < line.y_max as usize) {
            let boundary = field_boundary.height_at_pixel(line.x as f32) as usize;
            // println!("y: {}, boundary: {}", y, boundary);

            if *y < boundary {
                continue;
            }

            let point = yuyv.row(*y).nth(line.x as usize).unwrap();

            let color = PixelColor::classify_yuv_pixel(&field, point.y, point.u, point.v);

            scan_line_points.push((line.x as usize, *y, color));
        }
    }

    // println!("scan_line_points: {:?}", scan_line_points.len());

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

    // *scan_lines = ScanLines;

    Ok(())
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

impl PixelColor {
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

    pub fn classify_yuv_pixel(field: &FieldColorApproximate, y: u8, u: u8, v: u8) -> Self {
        let (y, h, s) = Self::yuv_to_yhs(y, u, v);

        let (lum_mean, lum_std) = field.luminance;

        let lum_diff = y - lum_mean;

        // dbg!(lum_diff, y, lum_mean, lum_std);

        if Self::is_green(field, y, h, s) {
            return PixelColor::Green;
        }

        if lum_diff > 3.0 * lum_std {
            return PixelColor::White;
        } else if y < 3.0 * lum_std {
            return PixelColor::Black;
        }

        PixelColor::Unknown
    }

    fn is_green(field: &FieldColorApproximate, y: f32, h: f32, s: f32) -> bool {
        let lum_diff = (field.luminance.0 - y).abs();
        let hue_diff = (field.hue.0 - h).abs();
        let sat_diff = (field.saturation.0 - s).abs();

        lum_diff < field.luminance.1 * 3.0
            && hue_diff < field.hue.1 * 3.0
            && sat_diff < field.saturation.1 * 3.0
    }
}
