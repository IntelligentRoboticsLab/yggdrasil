use std::{io, path::Path};

use linuxvideo::{
    format::{PixFormat, PixelFormat},
    stream::FrameProvider,
    uvc::UvcExt,
    Device,
};

use super::{Error, Result, YuyvImage};

/// The width of a NAO [`Image`].
const IMAGE_WIDTH: u32 = 1280;

/// The height of a NAO [`Image`].
const IMAGE_HEIGHT: u32 = 960;

/// Absolute path to the lower camera of the NAO.
const CAMERA_BOTTOM: &str = "/dev/video-bottom";

/// Absolute path to the upper camera of the NAO.
const CAMERA_TOP: &str = "/dev/video-top";

/// A wrapper around a [`Device`] that contains utilities to flip the image.
pub struct CameraDevice {
    device: Device,
}

impl CameraDevice {
    pub fn new<A>(device_path: A) -> Result<Self>
    where
        A: AsRef<Path>,
    {
        let device = Device::open(device_path)?;

        Ok(Self { device })
    }

    pub fn horizontal_flip(&self) -> Result<()> {
        let mut uvc_extension = UvcExt::new(&self.device);
        uvc_extension
            .horizontal_flip()
            .map_err(Error::HorizontalFlip)
    }

    pub fn vertical_flip(&self) -> Result<()> {
        let mut uvc_extension = UvcExt::new(&self.device);
        uvc_extension.vertical_flip().map_err(Error::VerticalFlip)
    }
}

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    camera: FrameProvider,
    width: usize,
    height: usize,
}

impl Camera {
    /// Create a new camera object.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    ///
    /// # Panics
    /// This function pannics if it cannot convert a `u32` value to `usize`.
    pub fn new(
        camera_device: CameraDevice,
        width: u32,
        height: u32,
        num_buffers: u32,
    ) -> Result<Self> {
        if num_buffers == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Camera must have at least one buffer",
            ))?;
        }

        let capture_device =
            camera_device
                .device
                .video_capture(PixFormat::new(width, height, PixelFormat::YUYV))?;
        if capture_device.format().pixel_format() != PixelFormat::YUYV {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Pixel formats other than YUYV are not supported",
            ))?;
        }
        let width = usize::try_from(capture_device.format().width()).unwrap();
        let height = usize::try_from(capture_device.format().height()).unwrap();

        let camera = capture_device
            .into_stream_num_buffers(num_buffers)?
            .into_frame_provider();

        let mut camera = Self {
            camera,
            width,
            height,
        };

        // Grab some images to startup the camera.
        // Without it, the first couple of images will return an empty buffer.
        for _ in 0..num_buffers {
            camera.get_yuyv_image()?;
        }

        Ok(camera)
    }

    /// Create a new camera object for the NAO's top camera.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new_nao_top(num_buffers: u32) -> Result<Self> {
        let camera_device = CameraDevice::new(CAMERA_TOP)?;
        // We need to rotate the top camera 180 degrees, because it's upside down in the robot.
        camera_device.horizontal_flip()?;
        camera_device.vertical_flip()?;

        Self::new(camera_device, IMAGE_WIDTH, IMAGE_HEIGHT, num_buffers)
    }

    /// Create a new camera object for the NAO's bottom camera.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new_nao_bottom(num_buffers: u32) -> Result<Self> {
        let camera_device = CameraDevice::new(CAMERA_BOTTOM)?;

        Self::new(camera_device, IMAGE_WIDTH, IMAGE_HEIGHT, num_buffers)
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let frame = self.camera.fetch_frame()?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn try_get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let frame = self.camera.try_fetch_frame()?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }
}
