use std::ops::Deref;

use miette::Result;
use tyr::prelude::*;

use crate::camera::{Image, TopImage};

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

pub struct Line {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

fn detect_lines(image: Image) -> Result<Vec<Line>> {
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
