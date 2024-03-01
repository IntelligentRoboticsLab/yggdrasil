use crate::prelude::*;

use serde::{Deserialize, Serialize};

pub mod scan_lines;

use scan_lines::{ScanLinesConfig, ScanLinesModule};

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(ScanLinesModule)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub scan_lines: ScanLinesConfig,
}
