mod camera;
pub use camera::{
    Camera, RgbImage, YuyvImage, CAMERA_BOTTOM, CAMERA_TOP, IMAGE_HEIGHT, IMAGE_WIDTH,
};

mod error;
pub use error::Result;
