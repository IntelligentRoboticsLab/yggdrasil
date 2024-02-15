use std::{io, path::Path};

use linuxvideo::{
    controls::Cid,
    format::{PixFormat, PixelFormat},
    stream::FrameProvider,
    uvc::UvcExt,
    Device,
};

use super::{Error, Result, YuyvImage};

/// The width of a NAO [`Image`].
const IMAGE_WIDTH: u32 = 640;

/// The height of a NAO [`Image`].
const IMAGE_HEIGHT: u32 = 480;

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

    /// Flip the image horizontally.
    pub fn horizontal_flip(&self) -> Result<()> {
        let mut uvc_extension = UvcExt::new(&self.device);
        uvc_extension
            .horizontal_flip()
            .map_err(Error::HorizontalFlip)
    }

    /// Flip the image vertically.
    pub fn vertical_flip(&self) -> Result<()> {
        let mut uvc_extension = UvcExt::new(&self.device);
        uvc_extension.vertical_flip().map_err(Error::VerticalFlip)
    }

    /// Enable or disable the autofocus.
    ///
    /// Default=false.
    pub fn set_autofocus(&mut self, enable: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::FOCUS_AUTO, enable as i32)?;

        Ok(())
    }

    /// Set the autofocus of the camera device.
    ///
    /// `value` is in range [0, 250], default=0, step=25.
    pub fn set_focus_absolute(&mut self, value: i32) -> Result<()> {
        self.device.write_control_raw(Cid::FOCUS_ABSOLUTE, value)?;

        Ok(())
    }

    /// Set the brightness of the camera device.
    ///
    /// `value` is in range [-255, 255], default=0, step=1.
    pub fn set_brightness(&mut self, value: i32) -> Result<()> {
        self.device.write_control_raw(Cid::BRIGHTNESS, value)?;

        Ok(())
    }

    /// Set the contrast of the camera device.
    ///
    /// `value` is in range [0, 255], default=32, step=1.
    pub fn set_contrast(&mut self, value: i32) -> Result<()> {
        self.device.write_control_raw(Cid::CONTRAST, value)?;

        Ok(())
    }

    /// Set the saturation of the camera device.
    ///
    /// `value` is in range [0, 255], default=64, step=1.
    pub fn set_saturation(&mut self, value: i32) -> Result<()> {
        self.device.write_control_raw(Cid::SATURATION, value)?;

        Ok(())
    }

    /// Set the hue of the camera device.
    ///
    /// `value` is in range [-180, 180], default=0, step=1.
    pub fn set_hue(&mut self, value: i32) -> Result<()> {
        self.device.write_control_raw(Cid::SATURATION, value)?;

        Ok(())
    }

    // Enable or disable the auto hue of the camera device.
    ///
    /// Default=true.
    pub fn set_hue_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::HUE_AUTO, enabled as i32)?;

        Ok(())
    }

    /// Enable or disable to auto white (temperature) balance.
    ///
    /// Default=true.
    pub fn set_auto_white_balance(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::AUTO_WHITE_BALANCE, enabled as i32)?;

        Ok(())
    }

    /// Set the white balance as a color temperature in Kelvin.
    ///
    /// `value` is in range [2500, 6500], default=2500, step=500.
    pub fn set_white_balance_temperature(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::WHITE_BALANCE_TEMPERATURE, value)?;

        Ok(())
    }

    /// Set the gain of the camera device.
    ///
    /// `value` is in range [0, 1023], default=16, step=1.
    pub fn set_gain(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::WHITE_BALANCE_TEMPERATURE, value)?;

        Ok(())
    }

    /// Set the sharpness of the camera device.
    ///
    /// `value` is in range [0, 9], default=4, step=1.
    pub fn set_sharpness(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::SHARPNESS, enabled as i32)?;

        Ok(())
    }

    /// Enable or disable the auto exposure.
    ///
    /// Default=true.
    pub fn set_exposure_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::EXPOSURE_AUTO, !enabled as i32)?;

        Ok(())
    }

    /// Set the exposure of the camera device.
    ///
    /// `value` is in range [0, 1048575], default=512, step=1.
    pub fn set_exposure_absolute(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::EXPOSURE_ABSOLUTE, value)?;

        Ok(())
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
