/// Holds the exposure weights for both cameras.
#[derive(Clone)]
pub struct ExposureWeights {
    /// The exposure weights for the top part of the image.
    pub top: ExposureWeightTable,
    /// The exposure weights for the bottom part of the image.
    pub bottom: ExposureWeightTable,
}

impl ExposureWeights {
    /// Initialise `ExposureWeights` constant values.
    ///
    /// # Arguments
    ///
    /// * `image_dims` - The dimensions of the image (width, height).
    ///
    /// # Returns
    ///
    /// A new instance of `ExposureWeights`.
    pub fn new(image_dims: (u32, u32)) -> Self {
        Self {
            // Top camera is likely to suffer from overexposure when standing near a window,
            // by setting the weights to be higher in the lower part of the image, we can reduce this.
            top: ExposureWeightTable::new(
                image_dims,
                [0, 0, 0, 0, 2, 2, 2, 2, 5, 5, 5, 5, 15, 15, 15, 15],
            ),

            // Bottom camera rarely suffers from overexposure, so we can set a constant weight.
            bottom: ExposureWeightTable::new(image_dims, [ExposureWeightTable::MAX_VALUE; 16]),
        }
    }
}

/// Represents a table of exposure weights.
#[derive(Clone)]
pub struct ExposureWeightTable {
    /// The top-left corner of the exposure weight window.
    window_start: [u32; 2],

    /// The bottom-right corner of the exposure weight window.
    window_end: [u32; 2],

    /// The exposure weights for the window in row-major order.
    weights: [u8; 16],
}

impl ExposureWeightTable {
    /// Creates a new instance of `ExposureWeightTable` with the given image dimensions and initial weights.
    ///
    /// # Arguments
    ///
    /// * `image_dims` - The dimensions of the image (width, height).
    /// * `initial_weights` - The initial exposure weights for the table.
    ///
    /// # Returns
    ///
    /// A new instance of `ExposureWeightTable`.
    pub fn new(image_dims: (u32, u32), initial_weights: [u8; 16]) -> Self {
        let (image_width, image_height) = image_dims;
        Self {
            window_start: [0; 2], // Change `.window_size()` if we ever change this.
            window_end: [image_width, image_height],
            weights: initial_weights,
        }
    }

    /// Gets the size of the window.
    ///
    /// Since we currently use the whole screen for exposure, this is the camera width and height.
    pub fn window_size(&self) -> (u32, u32) {
        (self.window_end[0], self.window_end[1])
    }

    /// The maximum value for an exposure weight.
    pub const MAX_VALUE: u8 = 15;

    /// Updates the exposure weights with the given weights.
    ///
    /// # Arguments
    ///
    /// * `weights` - The new exposure weights.
    ///
    /// # Returns
    ///
    /// `true` if the weights were changed, `false` otherwise.
    /// Resetting the weights in the driver will be required if `true` is returned.
    pub fn update(&mut self, weights: [u8; 16]) -> bool {
        let mut weights = weights;
        for weight in weights.iter_mut() {
            if *weight > Self::MAX_VALUE {
                *weight = Self::MAX_VALUE;
            }
        }

        if self.weights == weights {
            return false;
        }

        self.weights = weights;
        true
    }

    /// Converts the exposure weight table to the expected byte array format.
    ///
    /// Format: https://spl.robocup.org/wp-content/uploads/downloads/nao-v6-hints.pdf
    ///
    /// # Returns
    ///
    /// The exposure weight table encoded as a byte array.
    pub fn encode(&self) -> [u8; 17] {
        let mut bytes = [0; 17];

        // In the documentation, the first byte controls if is enabled or not.
        // However, the driver does not seem to care about this value.
        bytes[0] = 1u8;
        bytes[1] = (self.window_start[0] >> 8) as u8;
        bytes[2] = (self.window_start[0] & 0xFF) as u8;
        bytes[3] = (self.window_start[1] >> 8) as u8;
        bytes[4] = (self.window_start[1] & 0xFF) as u8;
        bytes[5] = (self.window_end[0] >> 8) as u8;
        bytes[6] = (self.window_end[0] & 0xFF) as u8;
        bytes[7] = (self.window_end[1] >> 8) as u8;
        bytes[8] = (self.window_end[1] & 0xFF) as u8;

        for i in (0..self.weights.len()).step_by(2) {
            bytes[9 + i / 2] = (self.weights[i + 1] << 4) | self.weights[i];
        }

        bytes
    }
}
