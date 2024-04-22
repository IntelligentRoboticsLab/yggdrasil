use crate::{camera::matrix::CameraMatrices, prelude::*};

use heimdall::YuyvImage;
use serde::{Deserialize, Serialize};

/// Module that generates scan-lines from taken NAO images.
///
/// This module provides the following resources to the application:
/// - [`ScanGrid`]
pub struct ScanLinesModule;

impl Module for ScanLinesModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app)
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

pub struct FieldColorApproximate {
    pub luminance: f32,
    pub saturation: f32,
}

pub struct ScanGrid;

const FIELD_APPROXIMATION_STEP_SIZE: usize = 8;

// eth yellow box
// chargers with robot

pub fn approximate_field_color(image: &YuyvImage) -> FieldColorApproximate {
    let height = image.height();

    let rows_to_check = [
        image.row(height * 3 / 8),
        image.row(height / 4),
        image.row(height / 8),
    ];

    let mut luminances = Vec::new();
    let mut saturations = Vec::new();

    for row in rows_to_check {
        for pixel in row.step_by(FIELD_APPROXIMATION_STEP_SIZE) {
            let (y, _h, s2) = pixel.to_yhs2();

            luminances.push(y);
            saturations.push(s2);
        }
    }

    let luminance = luminances.iter().sum::<f32>() / luminances.len() as f32;
    let saturation = saturations.iter().sum::<f32>() / saturations.len() as f32;

    FieldColorApproximate {
        luminance,
        saturation,
    }
}

#[system]
pub fn make_scan_grid(camera_matrix: &CameraMatrices) -> ScanGrid {
    // camera_matrix.top.
    ScanGrid
}
