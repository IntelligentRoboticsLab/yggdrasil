use miette::Result;
use tyr::prelude::*;

pub mod line_detection;

pub struct VisionModule;

/// This module provides the following modules to the application:
/// - [`LineDetectionModule`](line_detection::LineDetectionModule)
impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(line_detection::LineDetectionModule)
    }
}
