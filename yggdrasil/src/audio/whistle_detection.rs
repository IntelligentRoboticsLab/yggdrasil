use std::sync::Arc;

use rustfft::{num_complex::Complex, Fft, FftPlanner};
use serde::{Deserialize, Serialize};

use crate::{
    audio::audio_input::{AudioInput, NUMBER_OF_SAMPLES},
    ml::{MlModel, MlTask, MlTaskResource},
    prelude::*,
};

const BUFFER_INDICES: [usize; 2] = [0, 1];

const FFT_SIZE: usize = 512;
const HOP_SIZE: usize = 256;

const FFT_NYQUIST_SIZE: usize = FFT_SIZE / 2 + 1;
const WINDOWS: usize = (NUMBER_OF_SAMPLES - FFT_SIZE) / HOP_SIZE + 1;

pub struct WhistleDetectionModule;

impl Module for WhistleDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_ml_task::<WhistleDetectionModel>()?
            .init_config::<WhistleDetectionConfig>()?
            .add_system(detect_whistle)
            .add_resource(Resource::new(WhistleState::new()))
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
    pub gain: f32,
    pub threshold: f32,
    pub consecutive: usize,
}

impl Config for WhistleDetectionConfig {
    const PATH: &'static str = "whistle_detection.toml";
}

pub struct WhistleState {
    detections: usize,
    fft: Arc<dyn Fft<f32>>,
}

impl WhistleState {
    fn new() -> Self {
        let mut planner = FftPlanner::new();

        Self {
            detections: 0,
            fft: planner.plan_fft_forward(FFT_SIZE),
        }
    }
}

#[system]
fn detect_whistle(
    state: &mut WhistleState,
    model: &mut MlTask<WhistleDetectionModel>,
    audio_input: &AudioInput,
    config: &WhistleDetectionConfig,
) -> Result<()> {
    if !model.active() {
        let mut complex_buffer = [Complex::new(0.0, 0.0); FFT_SIZE];
        let mut real_buffer = [0.0; FFT_NYQUIST_SIZE];

        // Break up the buffer into windows and average them.
        for i in 0..WINDOWS {
            // Load window into FFT buffer.
            for (j, complex_sample) in complex_buffer.iter_mut().enumerate() {
                let mut sample = 0.0;

                // Average over all the channels.
                for k in BUFFER_INDICES {
                    sample += audio_input.buffer[k][HOP_SIZE * i + j];
                }

                sample /= BUFFER_INDICES.len() as f32;
                sample *= config.gain;
                *complex_sample = Complex::new(sample, 0.0);
            }

            // Compute FFT.
            state.fft.process(&mut complex_buffer);

            // Take the amplitude and ignore frequencies above the Nyquist limit.
            for (real_sample, complex_sample) in
                real_buffer.iter_mut().zip(complex_buffer.iter_mut())
            {
                *real_sample += complex_sample.norm();
            }
        }

        // Compute the average over all windows from the sum.
        for real_sample in real_buffer.iter_mut() {
            *real_sample /= WINDOWS as f32;
        }

        // Run the model.
        model.try_start_infer(&real_buffer)?;
    }

    if let Some(Ok(result)) = model.poll::<Vec<f32>>() {
        if result[0] >= config.threshold {
            state.detections += 1;

            if state.detections == config.consecutive {
                tracing::info!("Whistle detected");
            }
        } else {
            state.detections = 0;
        }
    }

    Ok(())
}
