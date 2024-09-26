use bevy::app::{PluginGroup, PluginGroupBuilder};
use heimdall::{Bottom, Top};
// use robot_detection::RobotDetectionModule;
// use scan_grid::ScanGridModule;
// use scan_lines::ScanLinesModule;
use serde::{Deserialize, Serialize};

// pub mod ball_detection;
pub mod camera;
// pub mod color;
pub mod field_boundary;
// pub mod field_marks;
// pub mod line;
// pub mod line_detection;
// pub mod robot_detection;
pub mod scan_grid;
// pub mod scan_lines;
// pub mod util;

// use field_boundary::FieldBoundaryModule;

// use self::ball_detection::BallDetectionModule;
// use self::field_marks::{FieldMarksConfig, FieldMarksModule};
// use self::line_detection::LineDetectionModule;

pub struct VisionPlugins;

impl PluginGroup for VisionPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(camera::CameraPlugin::<Top>::default())
            .add(camera::CameraPlugin::<Bottom>::default())
            .add(scan_grid::ScanGridPlugin)
            .add(field_boundary::FieldBoundaryPlugin);

        // we only update the exposure weights for the top camera, so it cannot be part of
        // the camera plugin.
        #[cfg(not(feature = "local"))]
        let builder = builder.add(camera::exposure_weights::ExposureWeightsPlugin);

        builder
    }
}

// impl Module for VisionModule {
//     fn initialize(self, app: App) -> Result<App> {
//         app.add_module(FieldBoundaryModule)?
//             .add_module(ScanGridModule)?
//             .add_module(ScanLinesModule)?
//             .add_module(LineDetectionModule)?
//             .add_module(BallDetectionModule)?
//             .add_module(FieldMarksModule)?
//             .add_module(RobotDetectionModule)
//     }
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VisionConfig {
    pub field_marks: FieldMarksConfig,
}
