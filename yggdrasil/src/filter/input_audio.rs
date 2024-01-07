use alsa::pcm::*;
use alsa::{Direction, ValueOr};
use miette::{IntoDiagnostic, Result};
use std::sync::{Arc, Mutex};
use tyr::prelude::*;

const SAMPLE_RATE: usize = 44100;
const NUMBER_OF_SAMPLES: usize = 2048;
const NUMBER_OF_CHANNELS: usize = 2;
const FORMAT: Format = Format::FloatLE;
const ACCESS: Access = Access::RWInterleaved;

/// Records the input from the four microphones on the Nao's head.
/// Stores the recorded audio samples in a vector.
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
    pub buffer: Arc<Mutex<Vec<f32>>>,
    pub device: Arc<Mutex<PCM>>,
}

impl InputAudio {
    /// Initialize PCM and add the necesarry hardware parameters.
    fn new() -> Result<Self> {
        let device = PCM::new("default", Direction::Capture, false).into_diagnostic()?;
        let device = Arc::new(Mutex::new(device));
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(
            NUMBER_OF_SAMPLES * NUMBER_OF_CHANNELS,
        )));
        let input_audio = Self { buffer, device };
        {
            let device_lock = input_audio.device.lock().unwrap();
            let hwp = HwParams::any(&device_lock).into_diagnostic()?;
            hwp.set_channels(NUMBER_OF_CHANNELS as u32)
                .into_diagnostic()?;
            hwp.set_rate_near(SAMPLE_RATE as u32, ValueOr::Nearest)
                .into_diagnostic()?;
            hwp.set_format(FORMAT).into_diagnostic()?;
            hwp.set_access(ACCESS).into_diagnostic()?;
            device_lock.hw_params(&hwp).into_diagnostic()?;
        }

        input_audio
            .device
            .lock()
            .expect("Failed to lock device.")
            .prepare()
            .into_diagnostic()?;
        Ok(input_audio)
    }
}

pub struct AudioSample;

/// Reads audio samples into a temp buffer and then copies these samples to a buffer that is
/// part of the InputAudio resource.
fn microphone_input(device: Arc<Mutex<PCM>>, buffer: Arc<Mutex<Vec<f32>>>) -> AudioSample {
    let io_device = device.lock().expect("Failed to lock device.");
    let io = io_device.io_f32().into_diagnostic().expect("Failed to io.");
    let mut interleaved_buffer = vec![0.0; NUMBER_OF_SAMPLES * NUMBER_OF_CHANNELS];
    io.readi(&mut interleaved_buffer).unwrap();
    *buffer.lock().expect("Failed to lock audio buffer.") = interleaved_buffer;
    AudioSample
}

/// Checks wether the microphone_input function can be dispatched. This is the case when the
/// function is done reading the microphone input and stored it as a resource.
#[system]
fn dispatch_buffer(task: &mut ComputeTask<AudioSample>, audio: &mut InputAudio) -> Result<()> {
    if task.active() {
        let Some(_sample) = task.poll() else {
            return Ok(());
        };
    }

    let device = audio.device.clone();
    let buffer = audio.buffer.clone();

    match task.try_spawn(move || microphone_input(device, buffer)) {
        Ok(_) => Ok(()),
        Err(Error::AlreadyActive) => Ok(()),
    }
}
