use crate::YuyvImage;

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

    pub fn update(&mut self, image: &YuyvImage) {
        let (image_width, image_height) = (image.width() as u32, image.height() as u32);
        
        // The image is divided into 16 equal-sized windows, and the exposure is determined by the amount of green in each window.
        // However for optimization purposes, I want to take only 16 samples from each window, and then average them.
        let window_width = image_width / 4;
        let window_height = image_height / 4;

        let mut amount_green = [0; 16];

        for window_y in (0..image_height).step_by(window_height as usize) {
            for window_x in (0..image_width).step_by(window_width as usize) {

                for y in (window_y..(window_y + window_height)).step_by((window_height / 16) as usize){
                    for x in (window_x..(window_x + window_width)).step_by((window_width / 16) as usize){
                        let pixel = image.get_pixel(x as usize, y as usize);

                        if (pixel.y > 45) && (pixel.u > 70) && (pixel.u < 160) && (pixel.v > 70) && (pixel.v < 160) {
                            amount_green[(window_y / window_height * 4 + window_x / window_width) as usize] += 1;
                        }
                    }
                }
            }
        }

        print!("[");
        for i in 0..16 {
            if i % 4 == 0 {
                print!("\n");
            }
            print!("{}, ", amount_green[i]);           
        }
        print!("]\n\n");

        self.weights = amount_green.map(|x| x);

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