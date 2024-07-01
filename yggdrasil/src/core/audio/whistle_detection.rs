use std::sync::Arc;

use nidhogg::types::{FillExt, LeftEar, RightEar};
use num::Zero;
use rustfft::{num_complex::Complex, FftPlanner};
use serde::{Deserialize, Serialize};

use crate::{
    core::audio::audio_input::{AudioInput, NUMBER_OF_SAMPLES},
    core::ml::{MlModel, MlTask, MlTaskResource},
    nao::manager::{NaoManager, Priority},
    prelude::*,
};

// TODO: prolly add to config
/// The size of each window in samples.
const WINDOW_SIZE: usize = 512;
/// The interval between each window in samples.
const HOP_SIZE: usize = 256;
/// The number of windows to take the mean of before sending the average to the model.
const MEAN_WINDOWS: usize = (NUMBER_OF_SAMPLES - WINDOW_SIZE) / HOP_SIZE + 1;

pub struct WhistleDetectionModule;

impl Module for WhistleDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_ml_task::<WhistleDetectionModel>()?
            .init_config::<WhistleDetectionConfig>()?
            .add_system(detect_whistle)
            .init_resource::<WhistleState>()
    }
}

pub struct WhistleDetectionModel;

impl MlModel for WhistleDetectionModel {
    type InputType = f32;
    type OutputType = f32;
    const ONNX_PATH: &'static str = "models/whistle_detection.onnx";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhistleDetectionConfig {
    pub threshold: f32,
    /// For how many detection cycles to listen for a whistle.
    pub detection_tries: usize,
    /// How many detections within `detection_tries` detection cycles are required to flag a whistle.
    pub detections_needed: usize,
}

impl Config for WhistleDetectionConfig {
    const PATH: &'static str = "whistle_detection.toml";
}

/// Short time fourier transform, which decomposes a signal into the energy levels for each frequency
/// for each timestep.
///
/// Manages internal state to avoid repeat allocations over multiple calls.
pub struct Stft {
    fft: Arc<dyn rustfft::Fft<f32>>,
    window_size: usize,
    hop_size: usize,

    /// Reusable internal complex fft output buffer.
    window_buff: Vec<Complex<f32>>,
    /// Reusable internal fft scratch buffer.
    window_scratch: Vec<Complex<f32>>,
}

impl Stft {
    pub fn new(window_size: usize, hop_size: usize) -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(window_size);
        let scratch_len = fft.get_inplace_scratch_len();

        Self {
            fft,
            window_size,
            hop_size,

            window_buff: vec![Complex::zero(); window_size],
            window_scratch: vec![Complex::zero(); scratch_len],
        }
    }

    /// Computes the short time fourier transform with hann window smoothing
    /// for `windows` windows starting from `offset`.
    pub fn compute(&mut self, audio_pwr: &[f32], offset: usize, windows: usize) -> Spectrogram {
        let mut fft_outputs = Vec::with_capacity(windows * self.fft.len());
        let unique_freqs = self.window_size / 2 + 1;

        // compute windowed fft for every window
        for i in 0..windows {
            fft_outputs.extend(self.windowed_fft(audio_pwr, offset + i * self.hop_size));
        }
        return Spectrogram {
            powers: fft_outputs,
            freq_bins: unique_freqs,
        };
    }

    /// Computes a single window of the fast fourier transform with hann window smoothing.
    /// Starts from `offset` within the audio array.
    fn windowed_fft(&mut self, audio_pwr: &[f32], offset: usize) -> impl Iterator<Item = f32> + '_ {
        // apply window smoothing
        for (i, w) in apodize::hanning_iter(self.window_size).enumerate() {
            self.window_buff[i] = Complex::new(audio_pwr[offset + i] * w as f32, 0.0);
        }

        // compute fft
        self.fft
            .process_with_scratch(&mut self.window_buff, &mut self.window_scratch);

        return self
            .window_buff
            .iter()
            .cloned()
            // ft result is symmetric, only first window_size / 2 + 1 samples are unique
            .take(self.window_size / 2 + 1)
            // square norm of complex fft output
            .map(|c| c.norm_sqr());
    }
}

/// Output of a [`Stft`]. That is, the energy level for each frequency for each timestep.
#[derive(Debug, Serialize)]
pub struct Spectrogram {
    /// The energy levels.
    pub powers: Vec<f32>,
    /// The number of frequencies per timestep.
    pub freq_bins: usize,
}

impl Spectrogram {
    /// Returns the mean of all windows.
    pub fn windows_mean(self) -> Self {
        let mut powers = self.powers[0..self.freq_bins].to_vec();
        for (i, p) in self.powers.iter().skip(self.freq_bins).enumerate() {
            powers[i % self.freq_bins] += p;
        }

        let windows = (self.powers.len() / self.freq_bins) as f32;
        for i in 0..powers.len() {
            powers[i] /= windows;
        }
        return Self {
            powers,
            freq_bins: self.freq_bins,
        };
    }
}

pub struct WhistleState {
    detections: Vec<bool>,
    stft: Stft,
}

impl Default for WhistleState {
    fn default() -> Self {
        Self {
            detections: Vec::new(),
            stft: Stft::new(WINDOW_SIZE, HOP_SIZE),
        }
    }
}

#[system]
fn detect_whistle(
    state: &mut WhistleState,
    model: &mut MlTask<WhistleDetectionModel>,
    audio_input: &AudioInput,
    config: &WhistleDetectionConfig,
    nao_manager: &mut NaoManager,
) -> Result<()> {
    // TODO: 'scrub' empty bits from input
    // TODO: average channels, take random or run separate? (HULKS choose arbitrary ear I believe)

    if !model.active() {
        // take audio of arbitrary ear
        let spectrogram = state
            .stft
            .compute(&audio_input.buffer[0], 0, MEAN_WINDOWS)
            .windows_mean();

        // run detection model
        model.try_start_infer(&spectrogram.powers)?;
    }

    // check if detection cycle has been completed
    if let Some(Ok(result)) = model.poll::<Vec<f32>>() {
        // resize state.detections if necessary
        state.detections.resize(config.detection_tries, false);

        state.detections.rotate_right(1);
        state.detections[0] = result[0] >= config.threshold;

        let detections = state.detections.iter().fold(0, |acc, e| acc + *e as usize);

        if detections >= config.detections_needed {
            tracing::info!("Whistle detected");
            nao_manager.set_left_ear_led(LeftEar::fill(1.0), Priority::High);
            nao_manager.set_right_ear_led(RightEar::fill(1.0), Priority::High);
        } else {
            nao_manager.set_left_ear_led(LeftEar::fill(0.0), Priority::High);
            nao_manager.set_right_ear_led(RightEar::fill(0.0), Priority::High);
        }
    }

    Ok(())
}
