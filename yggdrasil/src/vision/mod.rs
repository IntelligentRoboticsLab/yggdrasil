use crate::prelude::*;

use serde::{Deserialize, Serialize};

pub mod field_boundary;
pub mod line_detection;
pub mod scan_lines;

use field_boundary::FieldBoundaryModule;
use line_detection::LineDetectionModule;
use scan_lines::{ScanLinesConfig, ScanLinesModule};

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(ScanLinesModule)?
            .add_module(FieldBoundaryModule)?
            .add_module(LineDetectionModule)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub scan_lines: ScanLinesConfig,
}
