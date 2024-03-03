pub struct ExposureWeights {
    pub top_weights: ExposureWeightTable,
    pub bottom_weights: ExposureWeightTable,
}

impl ExposureWeights {
    pub fn new(image_dims: (u32, u32)) -> Self {
        Self {
            top_weights: ExposureWeightTable::new(image_dims),
            bottom_weights: ExposureWeightTable::new(image_dims),
        }
    }
}

pub struct ExposureWeightTable {
    pub enabled: bool,
    pub window_start: [u32; 2],
    pub window_end: [u32; 2],
    pub weights: [u8; 16],
}

impl ExposureWeightTable {
    pub fn new(image_dims: (u32, u32)) -> Self {
        let (image_width, image_height) = image_dims;
        Self {
            enabled: true,
            window_start: [0; 2],
            window_end: [image_width, image_height],
            weights: [1; 16],
        }
    }

    pub fn encode(&self) -> [u8; 17] {
        let mut bytes = [0; 17];

        bytes[0] = self.enabled as u8;
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