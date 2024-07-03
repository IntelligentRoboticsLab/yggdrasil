use std::path::Path;

use heimdall::YuvPlanarImage;
use image::RgbImage;
use miette::{miette, IntoDiagnostic};

#[cfg(feature = "alsa")]
use crate::core::audio::sound_manager::{Sound, SoundManager};

use crate::{
    nao::Cycle,
    prelude::*,
    sensor::button::HeadButtons,
    vision::scan_lines::{BottomScanGrid, PixelColor, ScanGrid, TopScanGrid},
};

pub struct PhotoModule;

impl Module for PhotoModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(take_photo))
    }
}

#[system]
pub fn take_photo(
    top_scangrid: &TopScanGrid,
    bottom_scangrid: &BottomScanGrid,
    head_buttons: &HeadButtons,
    cycle: &Cycle,
    #[cfg(feature = "alsa")] sounds: &SoundManager,
) -> Result<()> {
    // make a directory to store the images
    let cycle = cycle.0;

    if head_buttons.rear.is_tapped() {
        std::fs::create_dir_all(format!("./photos/cycle_{cycle}")).into_diagnostic()?;

        YuvPlanarImage::from_yuyv(top_scangrid.image().yuyv_image())
            .store_jpeg(format!("./photos/cycle_{cycle}/top_image.jpg"), 100)?;

        YuvPlanarImage::from_yuyv(bottom_scangrid.image().yuyv_image())
            .store_jpeg(format!("./photos/cycle_{cycle}/bottom_image.jpg"), 100)?;

        top_scangrid.store_vertical(format!(
            "./photos/cycle_{cycle}/top_image_vertical_scanlines.png",
        ))?;

        top_scangrid.store_horizontal(format!(
            "./photos/cycle_{cycle}/top_image_horizontal_scanlines.png",
        ))?;

        bottom_scangrid.store_vertical(format!(
            "./photos/cycle_{cycle}/bottom_image_vertical_scanlines.png",
        ))?;

        bottom_scangrid.store_horizontal(format!(
            "./photos/cycle_{cycle}/bottom_image_horizontal_scanlines.png",
        ))?;

        #[cfg(feature = "alsa")]
        sounds.play_sound(Sound::Shutter)?
    }

    Ok(())
}

fn make_image_vertical(grid: &ScanGrid) -> Option<RgbImage> {
    let vertical = grid.vertical();

    let width = vertical.line_ids().len();
    let height = grid.height();

    let buffer: Vec<_> = vertical
        .raw()
        .iter()
        .cloned()
        .flat_map(|pixel| match pixel {
            PixelColor::White => [255, 255, 255],
            PixelColor::Black => [0, 0, 0],
            PixelColor::Green => [0, 255, 0],
            PixelColor::Unknown => [100, 100, 100],
        })
        .collect();

    RgbImage::from_vec(height as u32, width as u32, buffer)
        .map(|image| image::imageops::rotate90(&image))
        .map(|image| image::imageops::flip_horizontal(&image))
}

fn make_image_horizontal(grid: &ScanGrid) -> Option<RgbImage> {
    let horizontal = grid.horizontal();

    let width = grid.width();
    let height = horizontal.line_ids().len();

    let buffer: Vec<_> = horizontal
        .raw()
        .iter()
        .cloned()
        .flat_map(|pixel| match pixel {
            PixelColor::White => [255u8, 255, 255],
            PixelColor::Black => [0, 0, 0],
            PixelColor::Green => [0, 255, 0],
            PixelColor::Unknown => [100, 100, 100],
        })
        .collect();

    RgbImage::from_vec(width as u32, height as u32, buffer)
}

trait StoreScanlines {
    fn store_vertical(&self, path: impl AsRef<Path>) -> crate::Result<()>;
    fn store_horizontal(&self, path: impl AsRef<Path>) -> crate::Result<()>;
}

impl StoreScanlines for ScanGrid {
    fn store_vertical(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let image = make_image_vertical(&self)
            .ok_or_else(|| miette!("Failed to create image from vertical scanlines"))?;
        image.save(path).into_diagnostic()
    }

    fn store_horizontal(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let image = make_image_horizontal(&self)
            .ok_or_else(|| miette!("Failed to create image from vertical scanlines"))?;
        image.save(path).into_diagnostic()
    }
}
