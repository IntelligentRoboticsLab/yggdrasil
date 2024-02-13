use crate::prelude::*;

pub mod scan_lines;

pub struct VisionModule;

impl Module for VisionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(scan_lines::ScanLinesModule)
    }
}
