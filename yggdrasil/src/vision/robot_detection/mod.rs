use crate::prelude::*;

use super::camera::Image;

mod anchor_generator;
mod bbox;
mod box_coder;

pub struct RobotDetectionModule;

impl Module for RobotDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app)
        // .add_ml_task::<RobotDetectionModel>()?
        // .add_startup_system(init_robot_detection)?
        // .add_system(detect_robots))
    }
}

#[derive(Debug, Clone)]
pub struct DetectedRobot {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A fitted field boundary from a given image
#[derive(Clone)]
pub struct RobotDetectionData {
    /// The fitted field boundary lines
    pub robots: Vec<DetectedRobot>,
    /// The image the boundary was predicted from
    pub image: Image,
}

/// For keeping track of the image that a robot detection was made from
struct RobotDetectionImage(Image);
