mod fourier;

use bevy::prelude::*;
use fourier::Stft;
use nidhogg::types::{FillExt, LeftEar, RightEar};
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

use crate::{
    nao::{NaoManager, Priority},
    prelude::*,
};

use super::audio_input::{AudioSamplesEvent, TOTAL_SAMPLES};
use ml::prelude::*;

// the constants below need to match the parameters used for training
/// The size of each window in samples.
const WINDOW_SIZE: usize = 512;
/// The interval between each window in samples.
const HOP_SIZE: usize = 256;
/// The number of windows to take the mean of before sending the average to the model.
const MEAN_WINDOWS: usize = (TOTAL_SAMPLES as usize - WINDOW_SIZE) / HOP_SIZE + 1;

/// Nyquist assumed by the model.
const NYQUIST: usize = 24001;

/// Min and max Hz frequencies that the model uses.
const MIN_FREQ: usize = 2000;
const MAX_FREQ: usize = 4000;

pub struct WhistleDetectionPlugin;

impl Plugin for WhistleDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<WhistleDetectionModel>();
        app.init_resource::<WhistleDetectionState>();
        app.init_resource::<WhistleDetections>();
        app.init_config::<WhistleDetectionConfig>();

        app.add_systems(
            Update,
            (detect_whistle, update_whistle_state)
                .chain()
                .run_if(task_finished::<WhistleDetections>),
        );
    }
}

pub struct WhistleDetectionModel;

impl MlModel for WhistleDetectionModel {
    type InputElem = f32;
    type OutputElem = f32;

    type InputShape = (Vec<f32>,);
    type OutputShape = (Vec<f32>,);

    const ONNX_PATH: &'static str = "models/whistle_detection.onnx";
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
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

#[derive(Resource)]
pub struct WhistleDetectionState {
    detections: Vec<bool>,
    stft: Stft,
}

impl Default for WhistleDetectionState {
    fn default() -> Self {
        Self {
            detections: Vec::new(),
            stft: Stft::new(WINDOW_SIZE, HOP_SIZE),
        }
    }
}

#[derive(Debug, Default, Resource)]
pub struct WhistleDetections {
    pub detections: Vec<f32>,
}

fn update_whistle_state(
    detections: Res<WhistleDetections>,
    mut detection_state: ResMut<WhistleDetectionState>,
    config: Res<WhistleDetectionConfig>,
    mut nao_manager: ResMut<NaoManager>,
) {
    // resize state.detections if necessary
    detection_state
        .detections
        .resize(config.detection_tries, false);

    if detections.detections.len() == 0 {
        return;
    }

    detection_state.detections.rotate_right(1);
    detection_state.detections[0] = detections.detections[0] >= config.threshold;

    let detections = detection_state
        .detections
        .iter()
        .fold(0, |acc, e| acc + *e as usize);

    if detections >= config.detections_needed {
        // state.detected = true;
        nao_manager.set_left_ear_led(LeftEar::fill(1.0), Priority::High);
        nao_manager.set_right_ear_led(RightEar::fill(1.0), Priority::High);
    } else {
        // state.detected = false;
        nao_manager.set_left_ear_led(LeftEar::fill(0.0), Priority::High);
        nao_manager.set_right_ear_led(RightEar::fill(0.0), Priority::High);
    }
}

fn detect_whistle(
    mut commands: Commands,
    mut detection_state: ResMut<WhistleDetectionState>,
    mut model: ResMut<ModelExecutor<WhistleDetectionModel>>,
    mut audio_sample: EventReader<AudioSamplesEvent>,
) {
    for AudioSamplesEvent { left, .. } in audio_sample.read() {
        info!(?left, "left ear buffer");
        let spectrogram = detection_state
            .stft
            .compute(&left, 0, MEAN_WINDOWS)
            .windows_mean();

        let min_i = MIN_FREQ * spectrogram.powers.len() / NYQUIST;
        let max_i = MAX_FREQ * spectrogram.powers.len() / NYQUIST;

        commands
            .infer_model(&mut model)
            .with_input(&(spectrogram.powers[min_i..(max_i + 1)].to_vec(),))
            .to_resource()
            .spawn(|result| {
                Some(WhistleDetections {
                    detections: result.0,
                })
            });
    }

    // if !model.active() {
    //     // take audio of arbitrary ear
    //     let spectrogram = detection_state
    //         .stft
    //         .compute(&audio_input.buffer[0], 0, MEAN_WINDOWS)
    //         .windows_mean();

    //     let min_i = MIN_FREQ * spectrogram.powers.len() / NYQUIST;
    //     let max_i = MAX_FREQ * spectrogram.powers.len() / NYQUIST;

    //     // run detection model
    //     model.try_start_infer(&spectrogram.powers[min_i..(max_i + 1)])?;
    // }

    // // check if detection cycle has been completed
    // if let Some(Ok(result)) = model.poll::<Vec<f32>>() {
    //     // resize state.detections if necessary
    //     detection_state
    //         .detections
    //         .resize(config.detection_tries, false);

    //     detection_state.detections.rotate_right(1);
    //     detection_state.detections[0] = result[0] >= config.threshold;

    //     let detections = detection_state
    //         .detections
    //         .iter()
    //         .fold(0, |acc, e| acc + *e as usize);

    //     if detections >= config.detections_needed {
    //         state.detected = true;
    //         nao_manager.set_left_ear_led(LeftEar::fill(1.0), Priority::High);
    //         nao_manager.set_right_ear_led(RightEar::fill(1.0), Priority::High);
    //     } else {
    //         state.detected = false;
    //         nao_manager.set_left_ear_led(LeftEar::fill(0.0), Priority::High);
    //         nao_manager.set_right_ear_led(RightEar::fill(0.0), Priority::High);
    //     }
    // }

    // Ok(())
}
