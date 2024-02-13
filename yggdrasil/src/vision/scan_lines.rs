use std::time::Instant;

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
    top_width: usize,
    top_height: usize,
    bottom_width: usize,
    bottom_height: usize,

    top_horizontal: Vec<u8>,
    top_vertical: Vec<u8>,
    top_last_executed: Instant,
    top_horizontal_ids: Vec<usize>,
    top_vertical_ids: Vec<usize>,

    bottom_horizontal: Vec<u8>,
    bottom_vertical: Vec<u8>,
    bottom_last_executed: Instant,
    bottom_horizontal_ids: Vec<usize>,
    bottom_vertical_ids: Vec<usize>,
}

impl ScanLines {
    pub fn top_timestamp(&self) -> Instant {
        self.top_last_executed
    }

    pub fn bottom_timestamp(&self) -> Instant {
        self.bottom_last_executed
    }

    pub fn raw_top_horizontal(&self) -> &[u8] {
        &self.top_horizontal
    }

    pub fn raw_top_vertical(&self) -> &[u8] {
        &self.top_vertical
    }

    pub fn raw_bottom_horizontal(&self) -> &[u8] {
        &self.bottom_horizontal
    }

    pub fn raw_bottom_vertical(&self) -> &[u8] {
        &self.bottom_vertical
    }

    pub fn top_horizontal_ids(&self) -> &Vec<usize> {
        &self.top_horizontal_ids
    }

    pub fn top_vertical_ids(&self) -> &Vec<usize> {
        &self.top_vertical_ids
    }

    pub fn bottom_horizontal_ids(&self) -> &Vec<usize> {
        &self.bottom_horizontal_ids
    }

    pub fn bottom_vertical_ids(&self) -> &Vec<usize> {
        &self.bottom_vertical_ids
    }

    pub fn top_horizontal_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.top_width * 4;

        &self.top_horizontal.as_slice()[offset..offset + self.top_width * 4]
    }

    pub fn top_vertical_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.top_height * 4;

        &self.top_horizontal.as_slice()[offset..offset + self.top_height * 4]
    }

    pub fn bottom_horizontal_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.bottom_width * 4;

        &self.bottom_horizontal.as_slice()[offset..offset + self.bottom_width * 4]
    }

    pub fn bottom_vertical_line(&self, line_id: usize) -> &[u8] {
        let offset = line_id * self.bottom_height * 4;

        &self.bottom_horizontal.as_slice()[offset..offset + self.bottom_height * 4]
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

fn update_top_scan_lines(top_image: &TopImage, scan_lines: &mut ScanLines) {
    let top_start = Instant::now();
    horizontal_scan_lines(
        top_image.yuyv_image(),
        &mut scan_lines.top_horizontal,
        &mut scan_lines.top_horizontal_ids,
    );
    eprintln!(
        "top_horizontal elapsed: {}us",
        top_start.elapsed().as_micros()
    );

    let top_start = Instant::now();
    vertical_scan_lines(
        top_image.yuyv_image(),
        &mut scan_lines.top_vertical,
        &mut scan_lines.top_vertical_ids,
    );
    eprintln!(
        "top_vertical elapsed:   {}us",
        top_start.elapsed().as_micros()
    );
}

fn update_bottom_scan_lines(bottom_image: &BottomImage, scan_lines: &mut ScanLines) {
    let bottom_start = Instant::now();
    horizontal_scan_lines(
        bottom_image.yuyv_image(),
        &mut scan_lines.bottom_horizontal,
        &mut scan_lines.bottom_horizontal_ids,
    );
    eprintln!(
        "bottom_horizontal elapsed: {}us",
        bottom_start.elapsed().as_micros()
    );

    let bottom_start = Instant::now();
    vertical_scan_lines(
        bottom_image.yuyv_image(),
        &mut scan_lines.bottom_vertical,
        &mut scan_lines.bottom_vertical_ids,
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
    let (bottom_horizontal_buffer_size, bottom_vertical_buffer_size) =
        calc_buffer_size(bottom_image);

    let mut scan_lines = ScanLines {
        top_width: top_image.yuyv_image().width(),
        top_height: top_image.yuyv_image().height(),
        bottom_width: bottom_image.yuyv_image().width(),
        bottom_height: bottom_image.yuyv_image().height(),

        top_horizontal: vec![0u8; top_horizontal_buffer_size],
        top_vertical: vec![0u8; top_vertical_buffer_size],
        top_last_executed: *top_image.timestamp(),
        top_horizontal_ids: Vec::with_capacity(
            top_image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL,
        ),
        top_vertical_ids: Vec::with_capacity(
            top_image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL,
        ),
        bottom_horizontal: vec![0u8; bottom_horizontal_buffer_size],
        bottom_vertical: vec![0u8; bottom_vertical_buffer_size],
        bottom_last_executed: *bottom_image.timestamp(),
        bottom_horizontal_ids: Vec::with_capacity(
            bottom_image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL,
        ),
        bottom_vertical_ids: Vec::with_capacity(
            bottom_image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL,
        ),
    };

    update_top_scan_lines(top_image, &mut scan_lines);
    update_bottom_scan_lines(bottom_image, &mut scan_lines);

    storage.add_resource(Resource::new(scan_lines))?;

    Ok(())
}

#[system]
pub fn scan_lines_system(
    scan_lines: &mut ScanLines,
    top_image: &TopImage,
    bottom_image: &BottomImage,
) -> Result<()> {
    if scan_lines.top_last_executed != *top_image.timestamp() {
        let top_start = Instant::now();
        update_top_scan_lines(top_image, scan_lines);
        eprintln!("top elapsed: {}us", top_start.elapsed().as_micros());

        scan_lines.top_last_executed = *top_image.timestamp();
    }

    if scan_lines.bottom_last_executed != *bottom_image.timestamp() {
        let bottom_start = Instant::now();
        update_bottom_scan_lines(bottom_image, scan_lines);
        eprintln!("bottom elapsed: {}us", bottom_start.elapsed().as_micros());

        scan_lines.bottom_last_executed = *bottom_image.timestamp();
    }

    Ok(())
}
