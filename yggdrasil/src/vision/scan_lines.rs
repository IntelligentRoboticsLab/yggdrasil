use core::panic;
use std::{ops::Deref, time::Instant};

use crate::{camera::TopImage, prelude::*};

use heimdall::YuyvImage;

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

pub enum ScanLineType {
    Small(ScanLines<320, 240>),
    Medium(ScanLines<640, 480>),
    Large(ScanLines<1280, 960>),
}

impl ScanLineType {
    /// #[must_use]
    pub const fn width(&self) -> u32 {
        match self {
            ScanLineType::Small(_) => 320,
            ScanLineType::Medium(_) => 640,
            ScanLineType::Large(_) => 1280,
        }
    }

    /// #[must_use]
    pub const fn height(&self) -> u32 {
        match self {
            ScanLineType::Small(_) => 240,
            ScanLineType::Medium(_) => 480,
            ScanLineType::Large(_) => 960,
        }
    }
}

#[derive(Default)]
pub struct ScanLines<const IMAGE_WIDTH: usize, const IMAGE_HEIGHT: usize> {
    pub horizontal: Vec<ScanLine<IMAGE_WIDTH>>,
    pub vertical: Vec<ScanLine<IMAGE_HEIGHT>>,
}

pub struct ScanLine<const T: usize> {
    id: usize,
    data: [u8; T],
}

impl<const T: usize> ScanLine<T> {
    pub fn id(&self) -> usize {
        self.id
    }
}

impl<const T: usize> Deref for ScanLine<T> {
    type Target = [u8; T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

fn horizontal_scan_lines<const T: usize>(yuyv_image: &YuyvImage, buffer: &mut [ScanLine<T>]) {
    const ROW_SCAN_LINE_INTERVAL: usize = 16;

    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for row_id in 0..yuyv_image.height() / ROW_SCAN_LINE_INTERVAL {
        let offset = (yuyv_image.width() * 2) * (row_id * ROW_SCAN_LINE_INTERVAL);

        buffer[row_id].id = row_id * ROW_SCAN_LINE_INTERVAL;

        buffer[row_id]
            .data
            .as_mut_slice()
            .copy_from_slice(&yuyv_image.deref()[offset..offset + yuyv_image.width() * 2]);
    }
}

fn vertical_scan_lines<const T: usize>(yuyv_image: &YuyvImage, buffer: &mut [ScanLine<T>]) {
    const COL_SCAN_LINE_INTERVAL: usize = 16;

    // Warning is disabled, because iterators are to slow here.
    #[allow(clippy::needless_range_loop)]
    for col_id in 0..yuyv_image.width() / COL_SCAN_LINE_INTERVAL {
        for row_id in 0..yuyv_image.height() {
            let offset = row_id * yuyv_image.width() * 2 + col_id * COL_SCAN_LINE_INTERVAL * 2;

            buffer[col_id].data[row_id * 4..row_id * 4 + 4]
                .copy_from_slice(&yuyv_image[offset..offset + 4]);
        }
    }
}

#[startup_system]
pub fn init_buffers(storage: &mut Storage, top_image: &TopImage) -> Result<()> {
    let scan_line_type = match (
        top_image.yuyv_image().width(),
        top_image.yuyv_image().height(),
    ) {
        (1280, 960) => {
            let buffer: ScanLines<1280, 960> = Default::default();
            ScanLineType::Large(buffer)
        }
        (640, 480) => {
            let buffer: ScanLines<640, 480> = Default::default();
            ScanLineType::Medium(buffer)
        }
        (320, 240) => {
            let buffer: ScanLines<320, 240> = Default::default();
            ScanLineType::Small(buffer)
        }
        (_, _) => panic!("Unsupported Image dimensions"),
    };

    storage.add_resource(Resource::new(scan_line_type))?;

    Ok(())
}

#[system]
pub fn scan_lines_system(
    previously_executed_at: &mut PreviouslyExecutedAt,
    scan_line_type: &mut ScanLineType,
    top_image: &TopImage,
) -> Result<()> {
    if previously_executed_at.horizontal != *top_image.timestamp() {
        match scan_line_type {
            ScanLineType::Small(scan_lines) => {
                horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.horizontal);
            }
            ScanLineType::Medium(scan_lines) => {
                horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.horizontal);
            }
            ScanLineType::Large(scan_lines) => {
                horizontal_scan_lines(top_image.yuyv_image(), &mut scan_lines.horizontal);
            }
        }

        previously_executed_at.horizontal = *top_image.timestamp();
    }

    if previously_executed_at.vertical != *top_image.timestamp() {
        match scan_line_type {
            ScanLineType::Small(scan_lines) => {
                vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.vertical);
            }
            ScanLineType::Medium(scan_lines) => {
                vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.vertical);
            }
            ScanLineType::Large(scan_lines) => {
                vertical_scan_lines(top_image.yuyv_image(), &mut scan_lines.vertical);
            }
        }

        previously_executed_at.vertical = *top_image.timestamp();
    }

    Ok(())
}
