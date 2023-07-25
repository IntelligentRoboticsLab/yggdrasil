//! Contains functions for simplifying the use of a V4L2
//!
//! Written for the Dutch Nao Team as part of project Yggdrasil
//! <https://github.com/intelligentroboticslab>
//!
//! by O. Bosgraaf

use std::{fs::File, io::Write, path::Path};

use crate::Result;

use linuxvideo::{
    format::{PixFormat, Pixelformat},
    stream::ReadStream,
    Device,
};

pub struct Camera {
    pub pix_format: PixFormat,
    camera_stream: ReadStream,
}

impl Camera {
    pub fn new_from_path(
        camera_path: &std::path::Path,
        requested_pix_format: PixFormat,
    ) -> Result<Self> {
        let video_capture = Device::open(&camera_path)?.video_capture(requested_pix_format)?;
        let pix_format = video_capture.format();

        Ok(Self {
            pix_format: PixFormat::new(
                pix_format.width(),
                pix_format.height(),
                pix_format.pixelformat(),
            ),
            camera_stream: video_capture.into_stream(3)?,
        })
    }

    pub fn new_from_device(camera: Device, requested_pix_format: PixFormat) -> Result<Self> {
        let video_capture = camera.video_capture(requested_pix_format)?;
        let pix_format = video_capture.format();

        Ok(Self {
            pix_format: PixFormat::new(
                pix_format.width(),
                pix_format.height(),
                pix_format.pixelformat(),
            ),
            camera_stream: video_capture.into_stream(4)?,
        })
    }

    /// Prints all video devices and their capabilities
    ///
    /// # Examples
    /// ```no_run
    /// use linuxvideo::Device;
    /// use heimdall::camera_handler::print_device_list;
    ///
    /// print_device_list();
    /// ```
    pub fn print_device_list() -> Result<()> {
        for device in linuxvideo::list()? {
            match device {
                Ok(device) => Self::list_capabilities(device)?,
                Err(e) => {
                    eprintln!("Skipping device due to error: {e:?}");
                }
            }
        }

        Ok(())
    }

    /// Lists camera device capabilities
    ///
    /// # Arguments
    /// * `device` -> a linuxvideo::Device
    ///
    /// # Examples
    /// ```no_run
    /// use linuxvideo::Device;
    /// use heimdall::camera_handler::{
    ///     new_device,
    ///     list_capabilities,
    /// };
    ///
    /// let path: String = String::from("/dev/video0");
    /// let device: Device = new_device(path);
    /// list_capabilities(device);
    /// ```
    pub fn list_capabilities(device: Device) -> Result<()> {
        let capabilities = device.capabilities()?;
        println!("- {}: {}", device.path()?.display(), capabilities.card());
        println!("  driver: {}", capabilities.driver());
        println!("  bus info: {}", capabilities.bus_info());
        println!(
            "  all capabilities:    {:?}",
            capabilities.all_capabilities()
        );
        println!(
            "  avail. capabilities: {:?}",
            capabilities.device_capabilities()
        );

        Ok(())
    }

    fn yuyv444_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
        fn clip(value: i32) -> u8 {
            i32::max(0, i32::min(255, value)) as u8
        }

        let c = y as i32 - 16;
        let d = u as i32 - 128;
        let e = v as i32 - 128;

        let red = (298 * c + 409 * e + 128) >> 8;
        let green = (298 * c - 100 * d - 208 * e + 128) >> 8;
        let blue = (298 * c + 516 * d + 128) >> 8;

        (clip(red), clip(green), clip(blue))
    }

    fn save_rgb_screenshot_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        let num_pixels = (self.pix_format.width() * self.pix_format.height()) as usize;

        let stream = &mut self.camera_stream;
        stream.dequeue(|image_buffer_yuv_422| {
            for pixel_duo_id in 0..(num_pixels / 2) {
                let input_offset: usize = (num_pixels / 2 - pixel_duo_id - 1) * 4;
                // let input_offset: usize = pixel_duo_id * 4;

                let y1 = image_buffer_yuv_422[input_offset];
                let u = image_buffer_yuv_422[input_offset + 1];
                let y2 = image_buffer_yuv_422[input_offset + 2];
                let v = image_buffer_yuv_422[input_offset + 3];

                let (red1, green1, blue1) = Self::yuyv444_to_rgb(y1, u, v);
                let (red2, green2, blue2) = Self::yuyv444_to_rgb(y2, u, v);

                destination.write_all(&[red2, green2, blue2, red1, green1, blue1])?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Save a raw RGB photo to the buffer.
    ///
    /// The buffer `destination` should have a size of at least WIDTH * HEIGHT * 3 bytes.
    pub fn save_rgb_screenshot(&mut self, destination: &mut [u8]) -> Result<()> {
        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_rgb_screenshot_from_yuyv(destination),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    pub fn save_rgb_screenshot_to_file(&mut self, destination: &Path) -> Result<()> {
        let output_file = File::create(destination)?;
        let mut output_file_buffer = std::io::BufWriter::with_capacity(4096, output_file);

        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_rgb_screenshot_from_yuyv(&mut output_file_buffer),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    fn save_greyscale_screenshot_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        let num_pixels = (self.pix_format.width() * self.pix_format.height()) as usize;

        self.camera_stream.dequeue(|image_buffer_yuv_422| {
            for pixel_duo_id in 0..(num_pixels / 2) {
                let input_offset: usize = (num_pixels / 2 - pixel_duo_id - 1) * 4;
                // let input_offset: usize = pixel_duo_id * 4;

                let y1 = image_buffer_yuv_422[input_offset];
                let y2 = image_buffer_yuv_422[input_offset + 2];

                destination.write_all(&[y1, y2])?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Save a greyscale photo to the buffer.
    ///
    /// The buffer `destination` should have a size of at least WIDTH * HEIGHT bytes.
    pub fn save_greyscale_screenshot(&mut self, destination: &mut [u8]) -> Result<()> {
        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_greyscale_screenshot_from_yuyv(destination),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    // pub fn save_greyscale_screenshot_to_file(&mut self, destination: &Path) -> Result<()> {
    //     let output_file = File::create(destination)?;
    //     let output_file_buffer = std::io::BufWriter::with_capacity(4096, output_file);
    //
    //     match self.pix_format.pixelformat() {
    //         Pixelformat::YUYV => self.save_greyscale_screenshot_from_yuyv(output_file_buffer),
    //         pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
    //     }
    // }
}
