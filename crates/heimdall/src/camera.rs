use std::{io, path::Path};

use linuxvideo::{
    controls::Cid,
    format::{PixFormat, PixelFormat},
    stream::FrameProvider,
    uvc::UvcExt,
    Device,
};

use crate::exposure_weights::ExposureWeightTable;

use super::{Error, Result, YuyvImage};

/// A wrapper around a [`Device`] that contains utilities to flip the image.
pub struct CameraDevice {
    device: Device,
}

impl CameraDevice {
    pub fn new<A>(device_path: A) -> Result<Self>
    where
        A: AsRef<Path>,
    {
        let path = device_path
            .as_ref()
            .to_owned()
            .to_string_lossy()
            .to_string();
        let device = Device::open_non_blocking(device_path)
            .map_err(|source| Error::DeviceOpen { path, source })?;

        Ok(Self { device })
    }

    fn try_clone(&self) -> Result<Self> {
        Ok(Self {
            device: self.device.try_clone()?,
        })
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

    /// Set the exposure weights of the camera device.
    pub fn set_auto_exposure_weights(&self, table: &ExposureWeightTable) -> Result<()> {
        let mut uvc_extension = UvcExt::new(&self.device);

        uvc_extension
            .set_auto_exposure_weights(&mut table.encode())
            .map_err(Error::SetAutoExposureWeights)
    }

    /// Enable or disable the autofocus.
    ///
    /// Default=false.
    pub fn set_focus_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::FOCUS_AUTO, enabled as i32)
            .map_err(|source| Error::DeviceProperty {
                property: "focus_auto".to_string(),
                value: enabled.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the focus of the camera device.
    ///
    /// `value` is in range [0, 250], default=0, step=25.
    pub fn set_focus_absolute(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::FOCUS_ABSOLUTE, value)
            .map_err(|source| Error::DeviceProperty {
                property: "focus_absolute".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the brightness of the camera device.
    ///
    /// `value` is in range [-255, 255], default=0, step=1.
    pub fn set_brightness(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::BRIGHTNESS, value)
            .map_err(|source| Error::DeviceProperty {
                property: "brightness".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the contrast of the camera device.
    ///
    /// `value` is in range [0, 255], default=32, step=1.
    pub fn set_contrast(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::CONTRAST, value)
            .map_err(|source| Error::DeviceProperty {
                property: "contrast".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the saturation of the camera device.
    ///
    /// `value` is in range [0, 255], default=64, step=1.
    pub fn set_saturation(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::SATURATION, value)
            .map_err(|source| Error::DeviceProperty {
                property: "saturation".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the hue of the camera device.
    ///
    /// `value` is in range [-180, 180], default=0, step=1.
    pub fn set_hue(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::HUE, value)
            .map_err(|source| Error::DeviceProperty {
                property: "hue".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    // Enable or disable the auto hue of the camera device.
    ///
    /// Default=true.
    pub fn set_hue_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::HUE_AUTO, enabled as i32)
            .map_err(|source| Error::DeviceProperty {
                property: "hue_auto".to_string(),
                value: enabled.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Enable or disable to auto white balance temperature.
    ///
    /// Default=true.
    pub fn set_white_balance_temperature_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::AUTO_WHITE_BALANCE, enabled as i32)
            .map_err(|source| Error::DeviceProperty {
                property: "white_balance_temperature_auto".to_string(),
                value: enabled.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the white balance as a color temperature in Kelvin.
    ///
    /// `value` is in range [2500, 6500], default=2500, step=500.
    pub fn set_white_balance_temperature(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::WHITE_BALANCE_TEMPERATURE, value)
            .map_err(|source| Error::DeviceProperty {
                property: "white_balance_temperature".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the gain of the camera device.
    ///
    /// `value` is in range [0, 1023], default=16, step=1.
    pub fn set_gain(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::GAIN, value)
            .map_err(|source| Error::DeviceProperty {
                property: "gain".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the sharpness of the camera device.
    ///
    /// `value` is in range [0, 9], default=4, step=1.
    pub fn set_sharpness(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::SHARPNESS, value)
            .map_err(|source| Error::DeviceProperty {
                property: "sharpness".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Enable or disable the auto exposure.
    ///
    /// Default=true.
    pub fn set_exposure_auto(&mut self, enabled: bool) -> Result<()> {
        self.device
            .write_control_raw(Cid::EXPOSURE_AUTO, !enabled as i32)
            .map_err(|source| Error::DeviceProperty {
                property: "exposure_auto".to_string(),
                value: enabled.to_string(),
                source,
            })?;

        Ok(())
    }

    /// Set the exposure of the camera device.
    ///
    /// `value` is in range [0, 1048575], default=512, step=1.
    pub fn set_exposure_absolute(&mut self, value: i32) -> Result<()> {
        self.device
            .write_control_raw(Cid::EXPOSURE_ABSOLUTE, value)
            .map_err(|source| Error::DeviceProperty {
                property: "exposure_absolute".to_string(),
                value: value.to_string(),
                source,
            })?;

        Ok(())
    }
}

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    camera: FrameProvider,
    device: CameraDevice,
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

        let capture_device = camera_device
            .try_clone()?
            .device
            .video_capture(PixFormat::new(width, height, PixelFormat::YUYV))
            .map_err(Error::VideoCapture)?;
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
            device: camera_device,
            camera,
            width,
            height,
        };

        // Grab some images to startup the camera.
        // Without it, the first couple of images will return an empty buffer.
        for _ in 0..num_buffers * 2 {
            camera.loop_try_get_yuyv_image()?;
        }

        Ok(camera)
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn try_get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let frame = self.camera.fetch_frame()?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }

    /// Get the next image.
    ///
    /// This is the same as `try_get_yuyv_image`, however this function infinite loops until it it
    /// has actually fetched an image. This can be useful when the camera device has been opened in
    /// non-blocking mode.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn loop_try_get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let mut fetch_frame_result = self.camera.fetch_frame();
        while fetch_frame_result
            .as_ref()
            .is_err_and(|io_error| io_error.kind() == std::io::ErrorKind::WouldBlock)
        {
            fetch_frame_result = self.camera.fetch_frame();
        }

        let frame = fetch_frame_result?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }

    pub fn camera_device(&self) -> &CameraDevice {
        &self.device
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}
