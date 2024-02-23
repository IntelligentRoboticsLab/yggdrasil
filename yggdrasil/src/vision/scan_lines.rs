use crate::{
    camera::{BottomImage, Image, TopImage},
    prelude::*,
};

use heimdall::YuyvImage;

use std::ops::Deref;

pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(scan_lines_system)
            .add_startup_system(init_buffers)
    }
}

pub struct ScanLines {
    horizontal: Vec<PixelColor>,
    vertical: Vec<PixelColor>,
    image: Image,

    horizontal_ids: Vec<usize>,
    vertical_ids: Vec<usize>,
}

impl ScanLines {
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

    /// Return a slice over all the horizontal scan lines.
    pub fn raw_horizontal(&self) -> &[PixelColor] {
        &self.horizontal
    }

    /// Return a slice over all the vertical scan lines.
    pub fn raw_vertical(&self) -> &[PixelColor] {
        &self.vertical
    }

    /// Return the id's of the rows from which a scan line was created.
    ///
    /// The row id's are sorted in ascending order, and therefore can be indexed by their
    /// corresponding horizontal scan line id.
    ///
    /// # Example
    /// ```ignore
    /// for horizontal_line_id in 0..scan_lines.row_ids() {
    ///     let row_id = scan_lines.row_ids()[horizontal_line_id];
    ///     let row = scan_lines.horizontal_line(horizontal_line_id);
    /// }
    /// ```
    pub fn row_ids(&self) -> &[usize] {
        &self.horizontal_ids
    }

    /// Return the id's of the columns from which a scan line was created.
    ///
    /// The column id's are sorted in ascending order, and therefore can be indexed by their
    /// corresponding vertical scan line id.
    ///
    /// # Example
    /// ```ignore
    /// for vertical_line_id in 0..scan_lines.column_ids() {
    ///     let column_id = scan_lines.column_ids()[vertical_line_id];
    ///     let column = scan_lines.vertical_line(vertical_line_id);
    /// }
    /// ```
    pub fn column_ids(&self) -> &[usize] {
        &self.vertical_ids
    }

    /// Return the horizontal scan line with scan line id `line_id`.
    pub fn horizontal_line(&self, line_id: usize) -> &[PixelColor] {
        let offset = line_id * self.width();

        &self.horizontal.as_slice()[offset..offset + self.width()]
    }

    /// Return the vertical scan line with scan line id `line_id`.
    pub fn vertical_line(&self, line_id: usize) -> &[PixelColor] {
        let offset = line_id * self.height();

        &self.vertical.as_slice()[offset..offset + self.height()]
    }
}

pub struct TopScanLines {
    scan_lines: ScanLines,
}

impl Deref for TopScanLines {
    type Target = ScanLines;

    fn deref(&self) -> &Self::Target {
        &self.scan_lines
    }
}

pub struct BottomScanLines {
    scan_lines: ScanLines,
}

impl Deref for BottomScanLines {
    type Target = ScanLines;

    fn deref(&self) -> &Self::Target {
        &self.scan_lines
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
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

fn calc_buffer_size(
    image: &Image,
    horizontal_ids: &[usize],
    vertical_ids: &[usize],
) -> (usize, usize) {
    let horizontal_buffer_size = image.yuyv_image().width() * horizontal_ids.len();
    let vertical_buffer_size = image.yuyv_image().height() * vertical_ids.len();

    (horizontal_buffer_size, vertical_buffer_size)
}

/// TODO: Make this configurable using Odal.
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

fn init_scan_lines(image: &Image) -> ScanLines {
    let horizontal_ids = make_horizontal_ids(image);
    let vertical_ids = make_vertical_ids(image);
    let (horizontal_buffer_size, vertical_buffer_size) =
        calc_buffer_size(image, &horizontal_ids, &vertical_ids);

    ScanLines {
        horizontal: vec![PixelColor::Unknown; horizontal_buffer_size],
        vertical: vec![PixelColor::Unknown; vertical_buffer_size],
        image: image.clone(),
        horizontal_ids,
        vertical_ids,
    }
}

#[startup_system]
fn init_buffers(
    storage: &mut Storage,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    let mut top_scan_lines = TopScanLines {
        scan_lines: init_scan_lines(top_image),
    };

    let mut bottom_scan_lines = BottomScanLines {
        scan_lines: init_scan_lines(bottom_image),
    };

    update_scan_lines(top_image, &mut top_scan_lines.scan_lines);
    update_scan_lines(bottom_image, &mut bottom_scan_lines.scan_lines);

    storage.add_resource(Resource::new(top_scan_lines))?;
    storage.add_resource(Resource::new(bottom_scan_lines))?;

    Ok(())
}

fn horizontal_scan_lines(yuyv_image: &YuyvImage, scan_lines: &mut ScanLines) {
    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for line_id in 0..scan_lines.row_ids().len() {
        let row_id = *unsafe { scan_lines.row_ids().get_unchecked(line_id) };

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
                *scan_lines.horizontal.get_unchecked_mut(buffer_offset) = pixel_color;
                *scan_lines.horizontal.get_unchecked_mut(buffer_offset + 1) = pixel_color;
            };
        }
    }
}

fn vertical_scan_lines(yuyv_image: &YuyvImage, scan_lines: &mut ScanLines) {
    // Warning is disabled, because iterators are too slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() {
        for line_id in 0..scan_lines.column_ids().len() {
            let col_id = *unsafe { scan_lines.column_ids().get_unchecked(line_id) };
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
                *scan_lines.vertical.get_unchecked_mut(buffer_offset) = pixel_color;
            };
        }
    }
}

fn update_scan_lines(image: &Image, scan_lines: &mut ScanLines) {
    horizontal_scan_lines(image.yuyv_image(), scan_lines);

    vertical_scan_lines(image.yuyv_image(), scan_lines);
}

#[system]
pub fn scan_lines_system(
    top_scan_lines: &mut TopScanLines,
    bottom_scan_lines: &mut BottomScanLines,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    if top_scan_lines.image.timestamp() != top_image.timestamp() {
        update_scan_lines(top_image, &mut top_scan_lines.scan_lines);

        top_scan_lines.scan_lines.image = top_image.deref().clone();
    }

    if bottom_scan_lines.image.timestamp() != bottom_image.timestamp() {
        update_scan_lines(bottom_image, &mut bottom_scan_lines.scan_lines);

        bottom_scan_lines.scan_lines.image = bottom_image.deref().clone();
    }
    Ok(())
}
