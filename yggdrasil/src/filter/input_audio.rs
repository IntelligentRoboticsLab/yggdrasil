use alsa::pcm::*;
use alsa::{Direction, ValueOr};
use miette::{IntoDiagnostic, Result};
use std::sync::{Arc, Mutex};
use tyr::prelude::*;

const SAMPLE_RATE: usize = 44100;
const NUMBER_OF_SAMPLES: usize = 2048;
const NUMBER_OF_CHANNELS: usize = 2;
const BUFSIZE: usize = NUMBER_OF_SAMPLES * NUMBER_OF_CHANNELS;
const FORMAT: Format = Format::FloatLE;
const ACCESS: Access = Access::RWInterleaved;

/// A module providing the microphone input audio data in the form of an audio vector.
///
/// This module provides the following resources to the application:
/// - [`InputAudio`]
pub struct InputAudioFilter;

impl Module for InputAudioFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_task::<ComputeTask<AudioSample>>()?
            .add_system(dispatch_buffer)
            .add_resource(Resource::new(InputAudio::new()?))
    }
}

/// Contains a vector that stores the captured PCM audio data.
pub struct InputAudio {
    pub buffer: Arc<[f32; BUFSIZE]>,
    device: Arc<Mutex<PCM>>,
}

impl InputAudio {
    /// Initialize PCM and add the necesarry hardware parameters.
    fn new() -> Result<Self> {
        let device = PCM::new("default", Direction::Capture, false).into_diagnostic()?;
        let buffer = [0.0; BUFSIZE];
        let buffer = Arc::new(buffer);
        {
            let hwp = HwParams::any(&device).into_diagnostic()?;
            hwp.set_channels(NUMBER_OF_CHANNELS as u32)
                .into_diagnostic()?;
            hwp.set_rate_near(SAMPLE_RATE as u32, ValueOr::Nearest)
                .into_diagnostic()?;
            hwp.set_format(FORMAT).into_diagnostic()?;
            hwp.set_access(ACCESS).into_diagnostic()?;
            device.hw_params(&hwp).into_diagnostic()?;
        }
        let device = Arc::new(Mutex::new(device));
        let input_audio = Self { buffer, device };
        input_audio
            .device
            .lock()
            .expect("Failed to lock device.")
            .prepare()
            .into_diagnostic()?;
        Ok(input_audio)
    }
}

pub struct AudioSample(Arc<[f32; BUFSIZE]>);

/// Reads audio samples into a temp buffer and returns that buffer.
fn microphone_input(device: Arc<Mutex<PCM>>) -> AudioSample {
    let io_device = device.lock().expect("Failed to lock device.");
    let io = io_device.io_f32().into_diagnostic().expect("Failed to io.");
    let mut interleaved_buffer = [0.0; BUFSIZE];
    io.readi(&mut interleaved_buffer).unwrap();
    AudioSample(Arc::new(interleaved_buffer))
}

/// Checks wether the microphone_input function can be dispatched. This is the case when the
/// function is done reading the microphone input and stored it as a resource. It also copies
/// the buffer that is returned from the task to input_audio so it can be used as a resource.
#[system]
fn dispatch_buffer(
    task: &mut ComputeTask<AudioSample>,
    input_audio: &mut InputAudio,
) -> Result<()> {
    if task.active() {
        let Some(buf) = task.poll() else {
            return Ok(());
        };
        input_audio.buffer = buf.0;
    }
    let device = input_audio.device.clone();
    task.try_spawn(move || microphone_input(device))
        .into_diagnostic()?;
    Ok(())
}
