use std::{ops::Deref, time::Instant};

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
    pub top_horizontal: Vec<u8>,
    pub top_vertical: Vec<u8>,
    top_last_executed: Instant,

    pub bottom_horizontal: Vec<u8>,
    pub bottom_vertical: Vec<u8>,
    bottom_last_executed: Instant,
}

fn horizontal_scan_lines(yuyv_image: &YuyvImage, buffer: &mut [u8]) {
    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() / ROW_SCAN_LINE_INTERVAL {
        let buffer_offset = (row_id * yuyv_image.width()) * 4;
        let image_offset = (yuyv_image.width() * 2) * (row_id * ROW_SCAN_LINE_INTERVAL);

        buffer[buffer_offset..buffer_offset + yuyv_image.width() * 2].copy_from_slice(
            &yuyv_image.deref()[image_offset..image_offset + yuyv_image.width() * 2],
        );
    }
}

fn vertical_scan_lines(yuyv_image: &YuyvImage, buffer: &mut [u8]) {
    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for col_id in 0..yuyv_image.width() / COL_SCAN_LINE_INTERVAL {
        for row_id in 0..yuyv_image.height() {
            let buffer_offset = (col_id * yuyv_image.height() + row_id) * 4;
            let image_offset =
                row_id * yuyv_image.width() * 2 + col_id * COL_SCAN_LINE_INTERVAL * 2;

            buffer[buffer_offset..buffer_offset + 4]
                .copy_from_slice(&yuyv_image[image_offset..image_offset + 4]);
        }
    }
}

fn calc_buffer_size(image: &Image) -> (usize, usize) {
    let horizontal_buffer_size =
        image.yuyv_image().width() * 4 * (image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL);
    let vertical_buffer_size =
        image.yuyv_image().height() * 4 * (image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL);

    (horizontal_buffer_size, vertical_buffer_size)
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
        top_horizontal: vec![0u8; top_horizontal_buffer_size],
        top_vertical: vec![0u8; top_vertical_buffer_size],
        top_last_executed: *top_image.timestamp(),
        bottom_horizontal: vec![0u8; bottom_horizontal_buffer_size],
        bottom_vertical: vec![0u8; bottom_vertical_buffer_size],
        bottom_last_executed: *bottom_image.timestamp(),
    };

    horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.top_horizontal);
    vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.top_vertical);
    horizontal_scan_lines(bottom_image.yuyv_image(), &mut scan_lines.bottom_horizontal);
    vertical_scan_lines(bottom_image.yuyv_image(), &mut scan_lines.bottom_vertical);

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
        horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.top_horizontal);
        vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.top_vertical);

        scan_lines.top_last_executed = *top_image.timestamp();
    }

    if scan_lines.bottom_last_executed != *bottom_image.timestamp() {
        horizontal_scan_lines(bottom_image.yuyv_image(), &mut scan_lines.bottom_horizontal);
        vertical_scan_lines(bottom_image.yuyv_image(), &mut scan_lines.bottom_vertical);

        scan_lines.bottom_last_executed = *bottom_image.timestamp();
    }

    Ok(())
}
