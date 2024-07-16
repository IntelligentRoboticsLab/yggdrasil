use crate::prelude::*;

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
pub mod scan_lines2;

use field_boundary::FieldBoundaryModule;

use scan_lines::{ScanLinesConfig, ScanLinesModule};

use self::ball_detection::BallDetectionModule;
use self::field_marks::{FieldMarksConfig, FieldMarksModule};
use self::line_detection::LineDetectionModule;

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(FieldBoundaryModule)?
            // TODO: use the new one!
            .add_module(scan_grid::ScanGridModule)?
            .add_module(scan_lines2::ScanLinesModule)?
            .add_module(ScanLinesModule)?
            .add_module(LineDetectionModule)?
            .add_module(BallDetectionModule)?
            .add_module(FieldMarksModule)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub scan_lines: ScanLinesConfig,
    pub field_marks: FieldMarksConfig,
}
