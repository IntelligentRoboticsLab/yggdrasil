use alsa::pcm::*;
use alsa::{Direction, ValueOr};
use miette::{IntoDiagnostic, Result};
use std::sync::{Arc, Mutex};
use tyr::prelude::*;

/// The amount of samples in a second, typically 44100.
pub const SAMPLE_RATE: usize = 44100;
/// How many audio samples to record per channel.
pub const NUMBER_OF_SAMPLES: usize = 2048;
/// The NAO has 4 microphones but alsa records audio in stereo so you get two channels.
pub const NUMBER_OF_CHANNELS: usize = 2;
/// The audio samples are in 32 bit float with a little endian layout.
pub const FORMAT: Format = Format::FloatLE;
/// Alternate samples for the left and right channel (LRLRLR).
pub const ACCESS: Access = Access::RWInterleaved;

/// This module provides the following resources to the application:
/// - [`AudioInput`]
pub struct AudioInputFilter;

impl Module for AudioInputFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_task::<ComputeTask<AudioSample>>()?
            .add_system(dispatch_buffer)
            .add_resource(Resource::new(AudioInput::new()?))
    }
}

/// Contains a vector that stores the captured PCM audio data. The audio samples are stored
/// with [`Access::RWInterleaved`], which means alternating between the left and right
/// channel, e.g. 'LRLRLR'.
pub struct AudioInput {
    /// Buffer containing audio samples with access [`Access::RWInterleaved`], which means
    /// alternating samples between the left and right channel, e.g. 'LRLRLR'.
    pub buffer: Arc<Vec<Vec<f32>>>,
    device: Arc<Mutex<PCM>>,
}

impl AudioInput {
    /// Initialize PCM and add the necesarry hardware parameters.
    fn new() -> Result<Self> {
        let device = PCM::new("default", Direction::Capture, false).into_diagnostic()?;
        let buffer = vec![Vec::with_capacity(NUMBER_OF_SAMPLES); NUMBER_OF_CHANNELS];
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
        let audio_input = Self { buffer, device };
        audio_input
            .device
            .lock()
            .expect("Failed to lock device.")
            .prepare()
            .into_diagnostic()?;
        Ok(audio_input)
    }
}

struct AudioSample(Arc<Vec<Vec<f32>>>);

/// Reads audio samples into a temp buffer and returns that buffer.
fn microphone_input(device: Arc<Mutex<PCM>>) -> Result<AudioSample> {
    let io_device = device.lock().expect("Failed to lock device.");
    let io = io_device.io_f32().into_diagnostic().expect("Failed to io.");

    let mut interleaved_buffer = vec![0.0 as f32; NUMBER_OF_SAMPLES * NUMBER_OF_CHANNELS];
    let number_of_frames = io.readi(&mut interleaved_buffer).into_diagnostic()?;

    assert_eq!(number_of_frames, NUMBER_OF_SAMPLES);

    let mut non_interleaved_buffer =
        vec![Vec::with_capacity(NUMBER_OF_SAMPLES); NUMBER_OF_CHANNELS];

    for (channel_idx, non_interleaved_buffer) in non_interleaved_buffer.iter_mut().enumerate() {
        non_interleaved_buffer.extend(
            interleaved_buffer
                .iter()
                .skip(channel_idx)
                .step_by(NUMBER_OF_CHANNELS),
        );
    }

    Ok(AudioSample(Arc::new(non_interleaved_buffer)))
}

/// Checks wether the [`microphone_input`] function can be dispatched. This is the case when the
/// function is done reading the microphone input and stored it as a resource. It also copies
/// the buffer that is returned from the task to [`input_audio`] so it can be used as a resource.
#[system]
fn dispatch_buffer(
    task: &mut ComputeTask<AudioSample>,
    audio_input: &mut AudioInput,
) -> Result<()> {
    if task.active() {
        let Some(buf) = task.poll() else {
            return Ok(());
        };
        audio_input.buffer = buf.0;
    }

    // Immediately spawn task again, to prevent it from blocking main thread.
    let device = audio_input.device.clone();
    task.try_spawn(move || microphone_input(device).expect("Failed to get buffer."))
        .into_diagnostic()?;

    Ok(())
}
