pub mod detect_lines;
pub mod ransac;
pub mod segmentation;

use nalgebra::DMatrix;

use std::ops::Deref;

use miette::Result;
use tyr::prelude::*;

use crate::{
    camera::{Image, TopImage},
    vision::line_detection::detect_lines::plot_image,
};

pub struct LineDetectionModule;

/// This module provides the following resources to the application:
/// - <code>[Vec]<[Line]></code>
impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(detect_lines_system)
            .init_resource::<Vec<Line>>()?
            .add_task::<ComputeTask<Result<Vec<Line>>>>()
    }
}

#[derive(Clone)]
pub struct Line {
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
}

pub type YUVImage = DMatrix<(u8, u8, u8)>;

pub struct RansacConfig {
    pub min_samples: usize,
    pub residual_threshold: f64,
    pub max_trials: usize,
    pub min_inliers: usize,
}

pub struct LineDetectionConfig {
    // The percentage of field pixels for a row to be considered the field barrier
    pub field_barrier_percentage: f32,

    pub horizontal_splits: usize,
    pub vertical_splits: usize,

    pub ransac: RansacConfig,
}

/// TODO: Delete this function and use [`detect_lines::detect_lines`] directly.
fn detect_lines(image: Image) -> Result<Vec<Line>> {
    let config = LineDetectionConfig {
        field_barrier_percentage: 0.3,
        horizontal_splits: 128,
        vertical_splits: 160,

        ransac: RansacConfig {
            min_samples: 4,
            residual_threshold: 20.0,
            max_trials: 1000,
            min_inliers: 25,
        },
    };

    let lines = detect_lines::detect_lines(config, image.yuyv_image());

    plot_image(lines.clone(), image.yuyv_image())?;

    Ok(lines)
}

#[system]
fn detect_lines_system(
    lines: &mut Vec<Line>,
    top_image: &TopImage,
    line_detection_task: &mut ComputeTask<Result<Vec<Line>>>,
) -> Result<()> {
    if let Some(new_lines) = line_detection_task.poll() {
        *lines = new_lines?;
    }

    let image: Image = top_image.deref().clone();
    let _ = line_detection_task.try_spawn(|| detect_lines(image));

    Ok(())
}
