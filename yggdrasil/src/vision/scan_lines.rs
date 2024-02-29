use crate::{
    camera::{BottomImage, Image, TopImage},
    prelude::*,
};

use heimdall::YuyvImage;

use std::ops::{Deref, DerefMut};

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

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PixelColor {
    White,
    Black,
    Green,
    Unknown,
}

impl PixelColor {
    fn classify_yuv_pixel(y1: u8, u: u8, _y2: u8, v: u8) -> Self {
        // TODO: Find a better way to classify pixels.
        if y1 > 140 {
            Self::White
        } else if (y1 > 45) && (u > 70) && (u < 160) && (v > 70) && (v < 160) {
            Self::Green
        } else if (y1 < 50) && (u > 110) && (u < 150) && (v > 110) && (v < 150) {
            Self::Black
        } else {
            Self::Unknown
        }
    }
}

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
    /// The scan lines were created from this image.
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

    fn build(image: &Image) -> ScanGrid {
        ScanGrid {
            horizontal: ScanLines::build_horizontal(image),
            vertical: ScanLines::build_vertical(image),
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

                let (y1, u, y2, v) = unsafe {
                    (
                        *yuyv_image.get_unchecked(image_offset),
                        *yuyv_image.get_unchecked(image_offset + 1),
                        *yuyv_image.get_unchecked(image_offset + 2),
                        *yuyv_image.get_unchecked(image_offset + 3),
                    )
                };

                let pixel_color = PixelColor::classify_yuv_pixel(y1, u, y2, v);
                let buffer_offset = line_id * yuyv_image.width() + col_id * 2;

                unsafe {
                    *self.horizontal.pixels.get_unchecked_mut(buffer_offset) = pixel_color;
                    *self.horizontal.pixels.get_unchecked_mut(buffer_offset + 1) = pixel_color;
                };
            }
        }
    }

    fn update_vertical(&mut self, yuyv_image: &YuyvImage) {
        for row_id in 0..yuyv_image.height() {
            for line_id in 0..self.vertical().line_ids().len() {
                let col_id = *unsafe { self.vertical().line_ids().get_unchecked(line_id) };
                let image_offset = (row_id * yuyv_image.width() + col_id) * 2;

                let (y1, u, y2, v) = unsafe {
                    (
                        *yuyv_image.get_unchecked(image_offset),
                        *yuyv_image.get_unchecked(image_offset + 1),
                        *yuyv_image.get_unchecked(image_offset + 2),
                        *yuyv_image.get_unchecked(image_offset + 3),
                    )
                };

                let pixel_color = PixelColor::classify_yuv_pixel(y1, u, y2, v);
                let buffer_offset = line_id * yuyv_image.height() + row_id;

                unsafe {
                    *self.vertical.pixels.get_unchecked_mut(buffer_offset) = pixel_color;
                };
            }
        }
    }
}

pub struct TopScanGrid {
    scan_grid: ScanGrid,
}

impl Deref for TopScanGrid {
    type Target = ScanGrid;

    fn deref(&self) -> &Self::Target {
        &self.scan_grid
    }
}

impl DerefMut for TopScanGrid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scan_grid
    }
}

pub struct BottomScanGrid {
    scan_grid: ScanGrid,
}

impl Deref for BottomScanGrid {
    type Target = ScanGrid;

    fn deref(&self) -> &Self::Target {
        &self.scan_grid
    }
}

impl DerefMut for BottomScanGrid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.scan_grid
    }
}

/// TODO: Make this configurable using Odal.
/// TODO: We want to sample more frequently higher up in the frame,
/// as lines there are further away and therefore smaller and harder to detect with a large sampling distance.
fn make_horizontal_ids(image: &Image) -> Vec<usize> {
    let mut horizontal_ids = Vec::new();

    for row_id in 0..image.yuyv_image().height() / 4 {
        if row_id % 8 == 0 {
            horizontal_ids.push(row_id);
        }
    }
    for row_id in image.yuyv_image().height() / 4..image.yuyv_image().height() / 2 {
        if row_id % 8 == 0 {
            horizontal_ids.push(row_id);
        }
    }
    for row_id in image.yuyv_image().height() / 2..image.yuyv_image().height() * 3 / 4 {
        if (row_id - 4) % 16 == 0 {
            horizontal_ids.push(row_id);
        }
    }
    for row_id in image.yuyv_image().height() * 3 / 4..image.yuyv_image().height() {
        if (row_id) % 32 == 0 {
            horizontal_ids.push(row_id);
        }
    }

    horizontal_ids
}

/// TODO: Make this configurable using Odal.
fn make_vertical_ids(image: &Image) -> Vec<usize> {
    const COL_SCAN_LINE_INTERVAL: usize = 16;

    let mut vertical_ids = Vec::new();

    for col_id in 0..image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL {
        vertical_ids.push(col_id * COL_SCAN_LINE_INTERVAL);
    }

    vertical_ids
}

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

    fn build_horizontal(image: &Image) -> Self {
        let ids = make_horizontal_ids(image);

        let buffer_size = Self::horizontal_buffer_size(image, &ids);
        let pixels = vec![PixelColor::Unknown; buffer_size];

        Self { pixels, ids }
    }

    fn build_vertical(image: &Image) -> Self {
        let ids = make_vertical_ids(image);

        let buffer_size = Self::vertical_buffer_size(image, &ids);
        let pixels = vec![PixelColor::Unknown; buffer_size];

        Self { pixels, ids }
    }

    /// Return a slice over all the scan lines.
    pub fn raw(&self) -> &[PixelColor] {
        &self.pixels
    }

    /// Return a slice of all the row/column id's from which the scan-lines have been created.
    /// The id's are sorted in ascending order, and therefore can be indexed by their
    /// corresponding scan-line id.
    ///
    /// # Example
    /// ```ignore
    /// for horizontal_line_id in 0..scan_lines.horizontal().ids() {
    ///     let row_id = scan_lines.horizontal().ids()[horizontal_line_id];
    ///     let row = scan_lines.horizontal().line(horizontal_line_id);
    /// }
    /// ```
    ///
    /// # Example
    /// ```ignore
    /// for vertical_line_id in 0..scan_lines.vertical().ids() {
    ///     let column_id = scan_lines.vertical().ids()[vertical_line_id];
    ///     let column = scan_lines.vertical().line(vertical_line_id);
    /// }
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
) -> Result<()> {
    let mut top_scan_lines = TopScanGrid {
        scan_grid: ScanGrid::build(top_image),
    };

    let mut bottom_scan_lines = BottomScanGrid {
        scan_grid: ScanGrid::build(bottom_image),
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
