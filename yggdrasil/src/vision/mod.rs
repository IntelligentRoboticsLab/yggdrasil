use crate::prelude::*;

use serde::{Deserialize, Serialize};

pub mod ball_detection;
pub mod field_boundary;
pub mod line_detection;
pub mod scan_lines;

use field_boundary::FieldBoundaryModule;
use line_detection::LineDetectionModule;
use scan_lines::{ScanLinesConfig, ScanLinesModule};

use self::ball_detection::BallDetectionModule;

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(FieldBoundaryModule)?
            .add_module(ScanLinesModule)?
            .add_module(LineDetectionModule)?
            .add_module(BallDetectionModule)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub scan_lines: ScanLinesConfig,
}
