use std::{ops::Deref, time::Instant};

use crate::{camera::TopImage, prelude::*};

use heimdall::YuyvImage;

const ROW_SCAN_LINE_INTERVAL: usize = 16;
const COL_SCAN_LINE_INTERVAL: usize = 16;

pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(scan_lines_system)
            .add_startup_system(init_buffers)?
            .add_resource(Resource::new(PreviouslyExecutedAt {
                horizontal: Instant::now(),
                vertical: Instant::now(),
            }))
    }
}

pub struct PreviouslyExecutedAt {
    horizontal: Instant,
    vertical: Instant,
}

#[derive(Default)]
pub struct ScanLines {
    pub horizontal: Vec<u8>,
    pub vertical: Vec<u8>,
}

fn horizontal_scan_lines(yuyv_image: &YuyvImage, buffer: &mut [u8]) {
    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() / ROW_SCAN_LINE_INTERVAL {
        let buffer_offset = (row_id * yuyv_image.width()) * 4;
        let image_offset = (yuyv_image.width() * 2) * (row_id * ROW_SCAN_LINE_INTERVAL);

        buffer[buffer_offset..buffer_offset + 4].copy_from_slice(
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

#[startup_system]
pub fn init_buffers(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let scan_lines = ScanLines {
        horizontal: vec![
            0u8;
            top_image.yuyv_image().width()
                * 4
                * (top_image.yuyv_image().height() / ROW_SCAN_LINE_INTERVAL)
        ],
        vertical: vec![
            0u8;
            top_image.yuyv_image().height()
                * 4
                * (top_image.yuyv_image().width() / COL_SCAN_LINE_INTERVAL)
        ],
    };

    storage.add_resource(Resource::new(scan_lines))?;

    Ok(())
}

#[system]
pub fn scan_lines_system(
    previously_executed_at: &mut PreviouslyExecutedAt,
    scan_lines: &mut ScanLines,
    top_image: &TopImage,
) -> Result<()> {
    if previously_executed_at.horizontal != *top_image.timestamp() {
        horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.horizontal);

        previously_executed_at.horizontal = *top_image.timestamp();
    }

    if previously_executed_at.vertical != *top_image.timestamp() {
        vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.vertical);

        previously_executed_at.vertical = *top_image.timestamp();
    }

    Ok(())
}
