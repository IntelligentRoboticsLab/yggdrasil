// TODO: Remove import after testing phase is over.
use image::codecs::jpeg::JpegEncoder;
// TODO: Remove this function after testing phase is over.
fn yuyv_to_rgb(
    source: &[u8],
    len: usize,
    mut destination: impl Write,
    rotate_180_degrees: bool,
) -> Result<()> {
    fn clamp(value: i32) -> u8 {
        value.clamp(0, 255) as u8
    }

    fn yuyv422_to_rgb(y1: u8, u: u8, y2: u8, v: u8) -> ((u8, u8, u8), (u8, u8, u8)) {
        let y1 = i32::from(y1) - 16;
        let u = i32::from(u) - 128;
        let y2 = i32::from(y2) - 16;
        let v = i32::from(v) - 128;

        let red1 = (298 * y1 + 409 * v + 128) >> 8;
        let green1 = (298 * y1 - 100 * u - 208 * v + 128) >> 8;
        let blue1 = (298 * y1 + 516 * u + 128) >> 8;

        let red2 = (298 * y2 + 409 * v + 128) >> 8;
        let green2 = (298 * y2 - 100 * u - 208 * v + 128) >> 8;
        let blue2 = (298 * y2 + 516 * u + 128) >> 8;

        (
            (clamp(red1), clamp(green1), clamp(blue1)),
            (clamp(red2), clamp(green2), clamp(blue2)),
        )
    }

    // Two pixels are stored in four bytes. Those four bytes are the y1, u, y2, v values in
    // that order. Because two pixels share the same u and v value, we decode both pixels at
    // the same time (using `yuyv422_to_rgb`), instead of one-by-one, to improve performance.
    //
    // A `pixel_duo` here refers to the two pixels with the same u and v values.
    // We iterate over all the pixel duo's in `source`, which is why we take steps of four
    // bytes.
    // for pixel_duo_id in 0..(source.len() / 4) {
    for pixel_duo_id in 0..len / 2 {
        let input_offset: usize = if rotate_180_degrees {
            source.len() - 4 * pixel_duo_id - 4
        } else {
            pixel_duo_id * 4
        };

        let y1 = source[input_offset];
        let u = source[input_offset + 1];
        let y2 = source[input_offset + 2];
        let v = source[input_offset + 3];

        let ((red1, green1, blue1), (red2, green2, blue2)) = yuyv422_to_rgb(y1, u, y2, v);

        if rotate_180_degrees {
            destination
                .write_all(&[red2, green2, blue2, red1, green1, blue1])
                .unwrap();
        } else {
            destination
                .write_all(&[red1, green1, blue1, red2, green2, blue2])
                .unwrap();
        }
    }

    Ok(())
}

// TODO: Remove this function after testing phase is over.
pub fn store_jpeg(
    image: Vec<u8>,
    width: usize,
    height: usize,
    file_path: impl AsRef<Path>,
) -> Result<()> {
    let output_file = File::create(file_path).unwrap();
    let mut encoder = JpegEncoder::new(output_file);

    let mut rgb_buffer = Vec::<u8>::with_capacity(width * height * 3);

    yuyv_to_rgb(&image, width * height, &mut rgb_buffer, false)?;

    encoder
        .encode(
            &rgb_buffer,
            u32::try_from(width).unwrap(),
            u32::try_from(height).unwrap(),
            image::ColorType::Rgb8,
        )
        .unwrap();

    Ok(())
}
use std::{fs::File, io::Write, ops::Deref, path::Path, process::exit, time::Instant};

use crate::{
    camera::{BottomImage, Image, TopImage},
    prelude::*,
};

use heimdall::YuyvImage;

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
    /// ```no_run
    /// for horizontal_line_id in 0..scan_lines.row_ids() {
    ///     let row_id = scan_lines.row_ids()[horizontal_line_id];
    ///     let row = scan_lines.horizontal_line(horizontal_line_id);
    /// }
    /// ```
    pub fn row_ids(&self) -> &Vec<usize> {
        &self.horizontal_ids
    }

    /// Return the id's of the columns from which a scan line was created.
    ///
    /// The column id's are sorted in ascending order, and therefore can be indexed by their
    /// corresponding vertical scan line id.
    ///
    /// # Example
    /// ```no_run
    /// for vertical_line_id in 0..scan_lines.column_ids() {
    ///     let column_id = scan_lines.column_ids()[vertical_line_id];
    ///     let column = scan_lines.vertical_line(vertical_line_id);
    /// }
    /// ```
    pub fn column_ids(&self) -> &Vec<usize> {
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
        // } else if (y1 > 65) && (u > 90) && (u < 110) && (v > 90) && (v < 135) {
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
        if row_id % 12 == 0 {
            horizontal_ids.push(row_id);
        }
    }
    for row_id in image.yuyv_image().height() / 2..image.yuyv_image().height() * 3 / 4 {
        if (row_id - 8) % 18 == 0 {
            horizontal_ids.push(row_id);
        }
    }
    for row_id in image.yuyv_image().height() * 3 / 4..image.yuyv_image().height() {
        if (row_id - 8) % 30 == 0 {
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
pub fn init_buffers(
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
        let row_id = scan_lines.row_ids()[line_id];

        for col_id in 0..yuyv_image.width() / 2 {
            let image_offset = (yuyv_image.width() * 2) * row_id + col_id * 4;

            let (y1, u, y2, v) = unsafe {
                (
                    yuyv_image.as_ptr().byte_add(image_offset).read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 1)
                        .read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 2)
                        .read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 3)
                        .read_unaligned(),
                )
            };

            let pixel_color = PixelColor::classify_yuv_pixel(y1, u, y2, v);
            let buffer_offset = line_id * yuyv_image.width() + col_id * 2;

            unsafe {
                scan_lines
                    .horizontal
                    .as_mut_ptr()
                    .byte_add(buffer_offset)
                    .write_unaligned(pixel_color);
                scan_lines
                    .horizontal
                    .as_mut_ptr()
                    .byte_add(buffer_offset + 1)
                    .write_unaligned(pixel_color);
            };
        }
    }
}

fn vertical_scan_lines(yuyv_image: &YuyvImage, scan_lines: &mut ScanLines) {
    // Warning is disabled, because iterators are too slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() {
        for line_id in 0..scan_lines.column_ids().len() {
            let col_id = scan_lines.column_ids()[line_id];
            let image_offset = (row_id * yuyv_image.width() + col_id) * 2;

            let (y1, u, y2, v) = unsafe {
                (
                    yuyv_image.as_ptr().byte_add(image_offset).read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 1)
                        .read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 2)
                        .read_unaligned(),
                    yuyv_image
                        .as_ptr()
                        .byte_add(image_offset + 3)
                        .read_unaligned(),
                )
            };

            let pixel_color = PixelColor::classify_yuv_pixel(y1, u, y2, v);
            let buffer_offset = line_id * yuyv_image.height() + row_id;

            unsafe {
                scan_lines
                    .vertical
                    .as_mut_ptr()
                    .byte_add(buffer_offset)
                    .write_unaligned(pixel_color)
            };
        }
    }
}

fn update_scan_lines(image: &Image, scan_lines: &mut ScanLines) {
    let horizontal_start = Instant::now();
    horizontal_scan_lines(image.yuyv_image(), scan_lines);
    // TODO: Remove this debug print.
    eprintln!("horizontal elapsed: {:?}", horizontal_start.elapsed());

    let vertical_start = Instant::now();
    vertical_scan_lines(image.yuyv_image(), scan_lines);
    // TODO: Remove this debug print.
    eprintln!("vertical elapsed: {:?}", vertical_start.elapsed());
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

    // TODO: Remove this. This stores the horizontal scan lines as a jpeg.
    {
        let mut row_yuyv_buffer = vec![0u8; top_scan_lines.width() * top_scan_lines.height() * 2];
        for (horizontal_id, row_id) in top_scan_lines.row_ids().iter().enumerate() {
            let row = top_scan_lines.horizontal_line(horizontal_id);

            let offset = row_id * top_scan_lines.width() * 2;

            for pixel_duo in 0..top_image.yuyv_image().width() / 2 {
                let yuyv_pixel_duo = match row[pixel_duo * 2] {
                    PixelColor::White => [128u8, 255u8, 128u8, 255u8],
                    PixelColor::Black => [128u8, 0u8, 128u8, 255u8],
                    PixelColor::Green => [128u8, 255u8, 128u8, 0u8],
                    PixelColor::Unknown => [0u8, 0u8, 0u8, 0u8],
                };

                row_yuyv_buffer.as_mut_slice()[offset + pixel_duo * 4..offset + pixel_duo * 4 + 4]
                    .copy_from_slice(&yuyv_pixel_duo);
            }
        }
        store_jpeg(
            row_yuyv_buffer,
            top_scan_lines.width(),
            top_scan_lines.height(),
            "yggdrasil_row_image.jpeg",
        )?;
    }

    // TODO: Remove this. This stores the vertical scan lines as a jpeg.
    {
        let mut col_yuyv_buffer = vec![0u8; top_scan_lines.width() * top_scan_lines.height() * 2];
        for (vertical_id, col_id) in top_scan_lines.column_ids().iter().enumerate() {
            let col = top_scan_lines.vertical_line(vertical_id);

            for (row_id, pixel) in col.iter().enumerate() {
                let buffer_offset = (row_id * top_scan_lines.width() + col_id) * 2;

                let yuyv_pixel_duo = match pixel {
                    PixelColor::White => [128u8, 255u8, 128u8, 255u8],
                    PixelColor::Black => [128u8, 0u8, 128u8, 255u8],
                    PixelColor::Green => [128u8, 255u8, 128u8, 0u8],
                    PixelColor::Unknown => [0u8, 0u8, 0u8, 0u8],
                };

                col_yuyv_buffer.as_mut_slice()[buffer_offset..buffer_offset + 4]
                    .copy_from_slice(&yuyv_pixel_duo);
            }
        }
        store_jpeg(
            col_yuyv_buffer,
            top_scan_lines.width(),
            top_scan_lines.height(),
            "yggdrasil_col_image.jpeg",
        )?;
    }

    // TODO: Remove this.
    exit(0);

    Ok(())
}
