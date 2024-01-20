use std::ops::Deref;

/// An object that holds a YUYV NAO camera image.
pub struct RgbImage {
    pub(super) frame: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl RgbImage {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }
}

impl Deref for RgbImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}
