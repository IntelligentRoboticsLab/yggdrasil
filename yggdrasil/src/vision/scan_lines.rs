use crate::{
    camera::{BottomImage, Image, TopImage},
    prelude::*,
};

use super::VisionConfig;

use heimdall::YuyvImage;

use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`TopScanGrid`]
/// - [`BottomScanGrid`]
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(scan_lines_system)
            .add_startup_system(init_buffers)
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

impl PixelColor {
    pub fn yuv_to_yhs2(y1: u8, u: u8, v: u8) -> (f32, f32, f32) {
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

    pub fn yuyv_to_yhs2(y1: u8, u: u8, y2: u8, v: u8) -> ((f32, f32, f32), (f32, f32, f32)) {
        let y1 = y1 as i32;
        let u = u as i32;
        let y2 = y2 as i32;
        let v = v as i32;

        let v_normed = v - 128;
        let u_normed = u - 128;

        let h =
            fast_math::atan2(v_normed as f32, u_normed as f32) * std::f32::consts::FRAC_1_PI * 127.
                + 127.;
        let s1 = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y1 as f32;
        let s2 = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y2 as f32;

        ((y1 as f32, h, s1), (y2 as f32, h, s2))
    }

    pub fn classify_yuv_pixel(y1: u8, u: u8, v: u8) -> Self {
        let (y, h, s2) = Self::yuv_to_yhs2(y1, u, v);

        if y > 120. && s2 < 55. {
            Self::White
        } else if y < 80. && s2 < 40. {
            Self::Black
        } else if y < 120. && !(20.0..=250.0).contains(&h) && s2 > 45. {
            Self::Green
        } else {
            Self::Unknown
        }
    }

    pub fn classify_yuyv_pixel(y1: u8, u: u8, y2: u8, v: u8) -> (Self, Self) {
        let ((y1, h1, s1), (y2, h2, s2)) = Self::yuyv_to_yhs2(y1, u, y2, v);

        let first = if y1 > 120. && s1 < 55. {
            Self::White
        } else if y1 < 80. && s1 < 40. {
            Self::Black
        } else if y1 < 120. && !(20.0..=250.0).contains(&h1) && s1 > 45. {
            Self::Green
        } else {
            Self::Unknown
        };

        let second = if y2 > 120. && s2 < 55. {
            Self::White
        } else if y2 < 80. && s2 < 40. {
            Self::Black
        } else if y2 < 120. && !(20.0..=250.0).contains(&h2) && s2 > 45. {
            Self::Green
        } else {
            Self::Unknown
        };

        (first, second)
    }

    pub fn yuyv_is_white(y1: u8, u: u8, y2: u8, v: u8) -> bool {
        let y1 = y1 as i32;
        let u = u as i32;
        let y2 = y2 as i32;
        let v = v as i32;

        let v_normed = v - 128;
        let u_normed = u - 128;

        let s1 = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y1 as f32;
        let s2 = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y2 as f32;

        // (y1 > 120 && s1 < 45.) || (y2 > 120 && s2 < 45.0)
        (y1 > 80 && s1 < 55.) || (y2 > 80 && s2 < 55.0)
    }
}

/// The horizontal and vertical scan-lines for an image.
#[derive(Clone)]
pub struct ScanGrid {
    image: Image,
    horizontal: ScanLines,
    vertical: ScanLines,
}

impl ScanGrid {
    /// Return the number of pixels per row.
    pub fn width(&self) -> usize {
        self.image.yuyv_image().width()
    }

    /// Return the number of pixels per column.
    pub fn height(&self) -> usize {
        self.image.yuyv_image().height()
    }

    /// Return the original image.
    ///
    /// The scan-lines were created from this image.
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// Return the horizontal scan-lines.
    pub fn horizontal(&self) -> &ScanLines {
        &self.horizontal
    }

    /// Return the vertical scan-lines.
    pub fn vertical(&self) -> &ScanLines {
        &self.vertical
    }

    fn build(image: &Image, config: &ScanLinesConfig) -> ScanGrid {
        ScanGrid {
            horizontal: ScanLines::build_horizontal(image, config.horizontal_scan_line_interval),
            vertical: ScanLines::build_vertical(image, config.vertical_scan_line_interval),
            image: image.clone(),
        }
    }

    fn update_scan_lines(&mut self, image: &Image) {
        self.update_horizontal(image.yuyv_image());
        self.update_vertical(image.yuyv_image());

        self.image = image.clone();
    }

    fn update_horizontal(&mut self, yuyv_image: &YuyvImage) {
        for line_id in 0..self.horizontal().line_ids().len() {
            let row_id = *unsafe { self.horizontal().line_ids().get_unchecked(line_id) };

            for col_id in 0..yuyv_image.width() / 2 {
                let image_offset = (yuyv_image.width() * 2) * row_id + col_id * 4;

                let [y1, u, y2, v] = unsafe {
                    [
                        *yuyv_image.get_unchecked(image_offset),
                        *yuyv_image.get_unchecked(image_offset + 1),
                        *yuyv_image.get_unchecked(image_offset + 2),
                        *yuyv_image.get_unchecked(image_offset + 3),
                    ]
                };

                let (pixel_color1, pixel_color2) = PixelColor::classify_yuyv_pixel(y1, u, y2, v);
                let buffer_offset = line_id * yuyv_image.width() + col_id * 2;

                unsafe {
                    *self.horizontal.pixels.get_unchecked_mut(buffer_offset) = pixel_color1;
                    *self.horizontal.pixels.get_unchecked_mut(buffer_offset + 1) = pixel_color2;
                };
            }
        }
    }

    fn update_vertical(&mut self, yuyv_image: &YuyvImage) {
        for row_id in 0..yuyv_image.height() {
            for line_id in 0..self.vertical().line_ids().len() {
                let col_id = *unsafe { self.vertical().line_ids().get_unchecked(line_id) };
                let image_offset = (row_id * yuyv_image.width() + col_id) * 2;

                let [y1, u, v] = unsafe {
                    [
                        *yuyv_image.get_unchecked(image_offset),
                        *yuyv_image.get_unchecked(image_offset + 1),
                        *yuyv_image.get_unchecked(image_offset + 3),
                    ]
                };

                let pixel_color = PixelColor::classify_yuv_pixel(y1, u, v);
                let buffer_offset = line_id * yuyv_image.height() + row_id;

                unsafe {
                    *self.vertical.pixels.get_unchecked_mut(buffer_offset) = pixel_color;
                };
            }
        }
    }
}

#[derive(Deref, DerefMut)]
/// Scan grid for the top image.
/// See [`ScanGrid`] for more info.
pub struct TopScanGrid {
    scan_grid: ScanGrid,
}

#[derive(Deref, DerefMut)]
/// Scan grid for the bottom image.
/// See [`ScanGrid`] for more info.
pub struct BottomScanGrid {
    scan_grid: ScanGrid,
}

/// TODO: Make this configurable using Odal.
/// TODO: We want to sample more frequently higher up in the frame,
/// as lines there are further away and therefore smaller and harder to detect with a large sampling distance.
fn make_horizontal_ids(image: &Image, scan_line_interval: usize) -> Vec<usize> {
    let mut horizontal_ids = Vec::new();

    for row_id in 0..image.yuyv_image().height() / scan_line_interval {
        horizontal_ids.push(row_id * scan_line_interval);
    }

    horizontal_ids
}

/// TODO: Make this configurable using Odal.
fn make_vertical_ids(image: &Image, scan_line_interval: usize) -> Vec<usize> {
    let mut vertical_ids = Vec::new();

    for col_id in 0..image.yuyv_image().width() / scan_line_interval {
        vertical_ids.push(col_id * scan_line_interval);
    }

    vertical_ids
}

/// A set of scan-lines stored in row-major order, with the ids of the subsampled indices from the original image.
#[derive(Clone)]
pub struct ScanLines {
    pixels: Vec<PixelColor>,
    ids: Vec<usize>,
}

impl ScanLines {
    fn horizontal_buffer_size(image: &Image, horizontal_ids: &[usize]) -> usize {
        image.yuyv_image().width() * horizontal_ids.len()
    }

    fn vertical_buffer_size(image: &Image, vertical_ids: &[usize]) -> usize {
        image.yuyv_image().height() * vertical_ids.len()
    }

    fn build_horizontal(image: &Image, scan_line_interval: usize) -> Self {
        let ids = make_horizontal_ids(image, scan_line_interval);

        let buffer_size = Self::horizontal_buffer_size(image, &ids);
        let pixels = vec![PixelColor::Unknown; buffer_size];

        Self { pixels, ids }
    }

    fn build_vertical(image: &Image, scan_line_interval: usize) -> Self {
        let ids = make_vertical_ids(image, scan_line_interval);

        let buffer_size = Self::vertical_buffer_size(image, &ids);
        let pixels = vec![PixelColor::Unknown; buffer_size];

        Self { pixels, ids }
    }

    /// Return a slice over all the scan-lines.
    pub fn raw(&self) -> &[PixelColor] {
        &self.pixels
    }

    /// Return a slice of all the row/column ids from which the scan-lines have been created.
    /// The ids are sorted in ascending order, and therefore can be indexed by their
    /// corresponding scan-line id.
    ///
    /// # Example
    /// ```
    /// # use yggdrasil::vision::scan_lines::ScanLines;
    /// # fn loop_over_lines(horizontal_scan_lines: ScanLines) {
    /// for horizontal_line_id in 0..horizontal_scan_lines.line_ids().len() {
    ///     let row_id = horizontal_scan_lines.line_ids()[horizontal_line_id];
    ///     let row = horizontal_scan_lines.line(horizontal_line_id);
    /// }
    ///
    /// for (horizontal_line_id, row_id) in horizontal_scan_lines.line_ids().iter().enumerate() {
    ///     let row = horizontal_scan_lines.line(horizontal_line_id);
    /// }
    /// # }
    /// ```
    ///
    /// # Example
    /// ```
    /// # use yggdrasil::vision::scan_lines::ScanLines;
    /// # fn loop_over_lines(vertical_scan_lines: ScanLines) {
    /// for vertical_line_id in 0..vertical_scan_lines.line_ids().len() {
    ///     let column_id = vertical_scan_lines.line_ids()[vertical_line_id];
    ///     let column = vertical_scan_lines.line(vertical_line_id);
    /// }
    ///
    /// for (vertical_line_id, column_id) in vertical_scan_lines.line_ids().iter().enumerate() {
    ///     let column = vertical_scan_lines.line(vertical_line_id);
    /// }
    /// # }
    /// ```
    pub fn line_ids(&self) -> &[usize] {
        &self.ids
    }

    /// Return the scan-line with scan-line id `line_id`.
    pub fn line(&self, line_id: usize) -> &[PixelColor] {
        let line_length = self.pixels.len() / self.ids.len();
        let offset = line_id * line_length;

        &self.pixels.as_slice()[offset..offset + line_length]
    }
}

#[startup_system]
fn init_buffers(
    storage: &mut Storage,
    top_image: &TopImage,
    bottom_image: &BottomImage,
    config: &VisionConfig,
) -> Result<()> {
    let mut top_scan_lines = TopScanGrid {
        scan_grid: ScanGrid::build(top_image, &config.scan_lines),
    };

    let mut bottom_scan_lines = BottomScanGrid {
        scan_grid: ScanGrid::build(bottom_image, &config.scan_lines),
    };

    top_scan_lines.update_scan_lines(top_image);
    bottom_scan_lines.update_scan_lines(bottom_image);

    storage.add_resource(Resource::new(top_scan_lines))?;
    storage.add_resource(Resource::new(bottom_scan_lines))?;

    Ok(())
}

#[system]
pub fn scan_lines_system(
    top_scan_grid: &mut TopScanGrid,
    bottom_scan_grid: &mut BottomScanGrid,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    if top_scan_grid.image().timestamp() != top_image.timestamp() {
        top_scan_grid.update_scan_lines(top_image);
    }

    if bottom_scan_grid.image().timestamp() != bottom_image.timestamp() {
        bottom_scan_grid.update_scan_lines(bottom_image);
    }

    Ok(())
}
