use std::ops::Deref;

/// An object that holds an RGB NAO camera image.
pub struct RgbImage {
    pub(super) frame: Vec<u8>,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl RgbImage {
    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }
}

impl Deref for RgbImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}
