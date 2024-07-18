use crate::core::ml::MlTask;
use crate::prelude::*;

use camera::{BottomImage, TopImage};
use scan_grid::ScanGridModule;
use scan_lines::ScanLinesModule;
use serde::{Deserialize, Serialize};

pub mod ball_detection;
pub mod camera;
pub mod color;
pub mod field_boundary;
pub mod field_marks;
pub mod line;
pub mod line_detection;
pub mod scan_grid;
pub mod scan_lines;
pub mod util;

use field_boundary::{FieldBoundary, FieldBoundaryImage, FieldBoundaryModel, FieldBoundaryModule};

use self::ball_detection::BallDetectionModule;
use self::field_marks::{FieldMarksConfig, FieldMarksModule};
use self::line_detection::LineDetectionModule;

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(FieldBoundaryModule)?
            .add_module(ScanGridModule)?
            .add_module(ScanLinesModule)?
            .add_module(LineDetectionModule)?
            .add_module(BallDetectionModule)?
            .add_module(FieldMarksModule)?
            .add_task::<AsyncTask<VisionPipeline>>()
    }
}

// .add_system_chain((
//     detect_field_boundary.after(camera::camera_system),
//     log_boundary_points,
// )))

// .add_system(update_scan_grid.after(super::camera::camera_system))

// .add_system(scan_lines_system.after(super::scan_grid::update_scan_grid))

// .add_system(line_detection_system.after(super::scan_lines::scan_lines_system))

// // ball detection
// .add_system_chain((
//     ball_proposals_system.after(scan_lines::scan_lines_system),
//     log_proposals,
// ))

// .add_system(ball_detection_system.after(proposal::ball_proposals_system))

// .add_system(log_balls.after(classifier::ball_detection_system))
// .add_system(reset_eye_color.after(classifier::ball_detection_system))

// .add_system(field_marks_system.after(super::line_detection::line_detection_system))

struct VisionPipeline;

pub fn vision_pipeline(
    vision_task: ResMut<AsyncTask<VisionPipeline>>,
    boundary_task: ResMut<MlTask<FieldBoundaryModel>>,
    boundary_image: ResMut<FieldBoundaryImage>,
    boundary: ResMut<FieldBoundary>,
    (top_image, bottom_image): (Res<TopImage>, Res<BottomImage>),
) -> Result<()> {
    // vision_task.try_spawn(async {
    //     field_boundary::detect_field_boundary(boundary_task, boundary_image, boundary, top_image);

    //     VisionPipeline
    // })?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub field_marks: FieldMarksConfig,
}
