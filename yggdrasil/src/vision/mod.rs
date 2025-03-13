use bevy::app::{PluginGroup, PluginGroupBuilder};
use heimdall::{Bottom, Top};

pub mod ball_detection;
pub mod body_contour;
pub mod camera;
pub mod color;
pub mod field_boundary;
pub mod line_detection;
pub mod referee;
pub mod robot_detection;
pub mod scan_grid;
pub mod scan_lines;
pub mod util;

/// Group of all vision plugins.
pub struct VisionPlugins;

impl PluginGroup for VisionPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>()
            .add(camera::CameraPlugin::<Top>::default())
            .add(camera::CameraPlugin::<Bottom>::default())
            .add(body_contour::BodyContourPlugin)
            .add(scan_grid::ScanGridPlugin)
            .add(scan_lines::ScanLinesPlugin)
            .add(line_detection::LineDetectionPlugin::<Top>::default())
            .add(line_detection::LineDetectionPlugin::<Bottom>::default())
            .add(field_boundary::FieldBoundaryPlugin)
            .add(ball_detection::BallDetectionPlugin)
            // .add(robot_detection::RobotDetectionPlugin)
            .add(referee::VisualRefereePlugin);

        // we only update the exposure weights for the top camera, so it cannot be part
        // of the camera plugin.
        #[cfg(not(feature = "local"))]
        let builder = builder.add(camera::exposure_weights::ExposureWeightsPlugin);

        builder
    }
}
