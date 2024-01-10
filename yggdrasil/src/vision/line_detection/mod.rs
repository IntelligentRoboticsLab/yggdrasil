pub mod detect_lines;
pub mod ransac;
pub mod segmentation;

use nalgebra::DMatrix;

use std::{ops::Deref, time::Instant};

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

/// TODO: Delete this function and use [`detect_lines::detect_lines`] directly.
fn detect_lines(image: Image) -> Result<Vec<Line>> {
    let before = Instant::now();

    let lines = detect_lines::detect_lines(image.yuyv_image());

    println!("Elapsed time: {:.2?}", before.elapsed());

    plot_image(lines.clone(), image.yuyv_image()).unwrap();

    for line in lines {
        println!("{:?}, {:?}, {:?}, {:?}", line.x1, line.y1, line.x2, line.y2);
    }
    Ok(Vec::new())
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
