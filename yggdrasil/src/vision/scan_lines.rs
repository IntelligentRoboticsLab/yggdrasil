use image::codecs::jpeg::JpegEncoder;
fn yuyv_to_rgb(
    source: &[u8],
    len: usize,
    mut destination: impl Write,
    rotate_180_degrees: bool,
) -> Result<()> {
    fn clamp(value: i32) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        return value.clamp(0, 255) as u8;
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
use std::{fs::File, io::Write, ops::Deref, path::Path, time::Instant};

use crate::{
    camera::{BottomImage, Image, TopImage},
    prelude::*,
};

use heimdall::YuyvImage;

const ROW_SCAN_LINE_INTERVAL: usize = 16;
const COL_SCAN_LINE_INTERVAL: usize = 16;

pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(scan_lines_system)
            .add_startup_system(init_buffers)
    }
}

pub struct ScanLines {
    width: usize,
    height: usize,

    horizontal: Vec<u8>,
    vertical: Vec<u8>,
    last_executed: Instant,

    horizontal_ids: Vec<usize>,
    vertical_ids: Vec<usize>,
}

impl ScanLines {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn timestamp(&self) -> Instant {
        self.last_executed
    }

    pub fn raw_horizontal(&self) -> &[u8] {
        &self.horizontal
    }

    pub fn raw_vertical(&self) -> &[u8] {
        &self.vertical
    }

    pub fn horizontal_ids(&self) -> &Vec<usize> {
        &self.horizontal_ids
    }

    pub fn vertical_ids(&self) -> &Vec<usize> {
        &self.vertical_ids
    }

    pub fn horizontal_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.width * 2;

        &self.horizontal.as_slice()[offset..offset + self.width * 2]
    }

    pub fn vertical_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.height * 4;

        &self.vertical.as_slice()[offset..offset + self.height * 4]
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

fn horizontal_scan_lines(
    yuyv_image: &YuyvImage,
    buffer: &mut [u8],
    horizontal_ids: &mut Vec<usize>,
) {
    horizontal_ids.clear();

    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() / ROW_SCAN_LINE_INTERVAL {
        horizontal_ids.push(row_id);

        let image_offset = (yuyv_image.width() * 2) * (row_id * ROW_SCAN_LINE_INTERVAL);
        let buffer_offset = (row_id * yuyv_image.width()) * 2;

        unsafe {
            std::ptr::copy_nonoverlapping(
                yuyv_image.as_ptr().byte_add(image_offset),
                buffer.as_mut_ptr().byte_add(buffer_offset),
                yuyv_image.width() * 2,
            );
        }
    }
}

fn vertical_scan_lines(yuyv_image: &YuyvImage, buffer: &mut [u8], vertical_ids: &mut Vec<usize>) {
    vertical_ids.clear();

    // Warning is disabled, because iterators are too slow here.
    #[allow(clippy::needless_range_loop)]
    for col_id in 0..yuyv_image.width() / COL_SCAN_LINE_INTERVAL {
        vertical_ids.push(col_id);

        for row_id in 0..yuyv_image.height() {
            let image_offset = (row_id * yuyv_image.width() + col_id * COL_SCAN_LINE_INTERVAL) * 2;
            let buffer_offset = (col_id * yuyv_image.height() + row_id) * 4;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    yuyv_image.as_ptr().byte_add(image_offset),
                    buffer.as_mut_ptr().byte_add(buffer_offset),
                    4,
                );
            }
        }
    }
}

fn calc_buffer_size(image: &Image) -> (usize, usize) {
    let horizontal_buffer_size =
        image.yuyv_image().width() * 2 * (image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL);
    let vertical_buffer_size =
        image.yuyv_image().height() * 4 * (image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL);

    (horizontal_buffer_size, vertical_buffer_size)
}

fn update_top_scan_lines(top_image: &TopImage, top_scan_lines: &mut TopScanLines) {
    let top_start = Instant::now();
    horizontal_scan_lines(
        top_image.yuyv_image(),
        &mut top_scan_lines.scan_lines.horizontal,
        &mut top_scan_lines.scan_lines.horizontal_ids,
    );
    eprintln!(
        "top_horizontal elapsed: {}us",
        top_start.elapsed().as_micros()
    );

    let top_start = Instant::now();
    vertical_scan_lines(
        top_image.yuyv_image(),
        &mut top_scan_lines.scan_lines.vertical,
        &mut top_scan_lines.scan_lines.vertical_ids,
    );
    eprintln!(
        "top_vertical elapsed:   {}us",
        top_start.elapsed().as_micros()
    );
}

fn update_bottom_scan_lines(bottom_image: &BottomImage, bottom_scan_lines: &mut BottomScanLines) {
    let bottom_start = Instant::now();
    horizontal_scan_lines(
        bottom_image.yuyv_image(),
        &mut bottom_scan_lines.scan_lines.horizontal,
        &mut bottom_scan_lines.scan_lines.horizontal_ids,
    );
    eprintln!(
        "bottom_horizontal elapsed: {}us",
        bottom_start.elapsed().as_micros()
    );

    let bottom_start = Instant::now();
    vertical_scan_lines(
        bottom_image.yuyv_image(),
        &mut bottom_scan_lines.scan_lines.vertical,
        &mut bottom_scan_lines.scan_lines.vertical_ids,
    );
    eprintln!(
        "bottom_vertical elapsed:   {}us",
        bottom_start.elapsed().as_micros()
    );
}

#[startup_system]
pub fn init_buffers(
    storage: &mut Storage,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    let (top_horizontal_buffer_size, top_vertical_buffer_size) = calc_buffer_size(top_image);
    let mut top_scan_lines = TopScanLines {
        scan_lines: ScanLines {
            width: top_image.yuyv_image().width(),
            height: top_image.yuyv_image().height(),

            horizontal: vec![0u8; top_horizontal_buffer_size],
            vertical: vec![0u8; top_vertical_buffer_size],
            last_executed: *top_image.timestamp(),
            horizontal_ids: Vec::with_capacity(
                top_image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL,
            ),
            vertical_ids: Vec::with_capacity(
                top_image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL,
            ),
        },
    };

    let (bottom_horizontal_buffer_size, bottom_vertical_buffer_size) =
        calc_buffer_size(bottom_image);
    let mut bottom_scan_lines = BottomScanLines {
        scan_lines: ScanLines {
            width: bottom_image.yuyv_image().width(),
            height: bottom_image.yuyv_image().height(),

            horizontal: vec![0u8; bottom_horizontal_buffer_size],
            vertical: vec![0u8; bottom_vertical_buffer_size],
            last_executed: *bottom_image.timestamp(),
            horizontal_ids: Vec::with_capacity(
                bottom_image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL,
            ),
            vertical_ids: Vec::with_capacity(
                bottom_image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL,
            ),
        },
    };

    update_top_scan_lines(top_image, &mut top_scan_lines);
    update_bottom_scan_lines(bottom_image, &mut bottom_scan_lines);

    storage.add_resource(Resource::new(top_scan_lines))?;
    storage.add_resource(Resource::new(bottom_scan_lines))?;

    Ok(())
}

#[system]
pub fn scan_lines_system(
    top_scan_lines: &mut TopScanLines,
    bottom_scan_lines: &mut BottomScanLines,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    if top_scan_lines.last_executed != *top_image.timestamp() {
        let top_start = Instant::now();
        update_top_scan_lines(top_image, top_scan_lines);
        eprintln!("top elapsed: {}us", top_start.elapsed().as_micros());

        top_scan_lines.scan_lines.last_executed = *top_image.timestamp();

        let mut row_yuyv_buffer = vec![0u8; top_scan_lines.width() * top_scan_lines.height() * 2];
        for (row_id, _) in top_scan_lines.horizontal_ids().iter().enumerate() {
            let row = top_scan_lines.horizontal_line(row_id);
            let offset = row_id * top_scan_lines.width() * ROW_SCAN_LINE_INTERVAL * 2;

            row_yuyv_buffer.as_mut_slice()[offset..offset + top_scan_lines.width() * 2]
                .copy_from_slice(row);
        }
        store_jpeg(
            row_yuyv_buffer,
            top_scan_lines.width(),
            top_scan_lines.height(),
            "yggdrasil_row_image.jpeg",
        )?;

        // This is to test whether to creation of `cols_buffer` went correctly.
        let mut col_yuyv_buffer = vec![0u8; top_scan_lines.width() * top_scan_lines.height() * 2];
        for (col_id, _) in top_scan_lines.vertical_ids().iter().enumerate() {
            for row_id in 0..top_scan_lines.height() {
                let col = top_scan_lines.vertical_line(col_id);
                let offset =
                    row_id * top_scan_lines.width() * 2 + col_id * COL_SCAN_LINE_INTERVAL * 2;

                col_yuyv_buffer.as_mut_slice()[offset..offset + 4]
                    .copy_from_slice(&col[row_id * 4..row_id * 4 + 4]);
            }
        }
        store_jpeg(
            col_yuyv_buffer,
            top_scan_lines.width(),
            top_scan_lines.height(),
            "yggdrasil_col_image.jpeg",
        )?;

        std::process::exit(0);
    }

    if bottom_scan_lines.last_executed != *bottom_image.timestamp() {
        let bottom_start = Instant::now();
        update_bottom_scan_lines(bottom_image, bottom_scan_lines);
        eprintln!("bottom elapsed: {}us", bottom_start.elapsed().as_micros());

        bottom_scan_lines.scan_lines.last_executed = *bottom_image.timestamp();
    }

    Ok(())
}
