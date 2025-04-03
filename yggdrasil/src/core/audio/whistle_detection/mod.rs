mod fourier;

use std::sync::{Arc, Mutex};

use async_std::task::block_on;
use bevy::{
    prelude::*,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
};

use bifrost::broadcast::Deadline;
use fourier::Stft;
use nidhogg::types::{FillExt, LeftEar, RightEar};
use serde::{Deserialize, Serialize};
use tasks::conditions::task_finished;

use crate::{
    behavior::primary_state::PrimaryState,
    communication::{TeamCommunication, TeamMessage},
    nao::{NaoManager, Priority},
    prelude::*,
};

use super::audio_input::AudioSamplesEvent;
use ml::prelude::*;

// the constants below need to match the parameters used for training
/// The size of each window in samples.
const WINDOW_SIZE: usize = 512;
/// The interval between each window in samples.
const HOP_SIZE: usize = 256;
/// The number of windows to take the mean of before sending the average to the model.
const MEAN_WINDOWS: usize = 4;

/// Nyquist assumed by the model.
const NYQUIST: usize = 24001;

/// Min and max Hz frequencies that the model uses.
const MIN_FREQ: usize = 2000;
const MAX_FREQ: usize = 4000;

pub struct WhistleDetectionPlugin;

impl Plugin for WhistleDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_ml_model::<WhistleDetectionModel>()
            .init_resource::<Whistle>()
            .init_resource::<WhistleDetectionState>()
            .init_config::<WhistleDetectionConfig>()
            .add_systems(Update, spawn_whistle_preprocess_task)
            .add_systems(
                Update,
                (update_whistle_state, despawn_whistle_preprocessing_task, spawn_whistle_detection_model)
                    .chain()
                    .run_if(task_finished::<WhistleDetections>),
            )
            //.add_systems(
            //    Update,
            //    despawn_whistle_preprocessing_task
            //        .run_if(resource_exists_and_changed::<WhistleDetections>),
            //);
            ;
    }
}

/// Whistle detection model.
///
/// A simple mlp that takes the STFT of the audio signal and outputs a single value.
pub struct WhistleDetectionModel;

impl MlModel for WhistleDetectionModel {
    type Inputs = Vec<f32>;
    type Outputs = Vec<f32>;

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

#[derive(Default, Resource)]
pub struct Whistle {
    detected: bool,
}

impl Whistle {
    #[must_use]
    pub fn detected(&self) -> bool {
        self.detected
    }
}

#[derive(Resource)]
struct WhistleDetectionState {
    detections: Vec<bool>,
    stft: Arc<Mutex<Stft>>,
}

impl Default for WhistleDetectionState {
    fn default() -> Self {
        Self {
            detections: Vec::new(),
            stft: Arc::new(Mutex::new(Stft::new(WINDOW_SIZE, HOP_SIZE))),
        }
    }
}

#[derive(Debug, Default, Component)]
struct WhistleDetections {
    pub detections: Vec<f32>,
}

fn update_whistle_state(
    primary_state: Res<PrimaryState>,
    ear_detections: Query<&WhistleDetections>,
    mut whistle: ResMut<Whistle>,
    mut detection_state: ResMut<WhistleDetectionState>,
    config: Res<WhistleDetectionConfig>,
    mut nao_manager: ResMut<NaoManager>,
    mut tc: ResMut<TeamCommunication>,
) {
    // resize state.detections if necessary
    detection_state
        .detections
        .resize(config.detection_tries, false);

    let incoming_msg = tc
        .inbound_mut()
        .take_map(|_, _, msg| match msg {
            TeamMessage::DetectedWhistle => Some(()),
            _ => None,
        })
        .is_some();

    if incoming_msg {
        whistle.detected = true;
        nao_manager.set_left_ear_led(LeftEar::fill(1.0), Priority::High);
        nao_manager.set_right_ear_led(RightEar::fill(1.0), Priority::High);
        return;
    }

    whistle.detected = false;

    // Detect whistle for all ears
    for detections in ear_detections.iter() {
        detection_state.detections.rotate_right(1);
        detection_state.detections[0] = detections.detections[0] >= config.threshold;

        let detections = detection_state
            .detections
            .iter()
            .fold(0, |acc, e| acc + usize::from(*e));

        if detections >= config.detections_needed {
            whistle.detected = true;

            if *primary_state == PrimaryState::Set {
                // Send message to all teammates
                let msg = TeamMessage::DetectedWhistle;
                tc.outbound_mut()
                    .update_or_push_by(msg, Deadline::ASAP)
                    .expect("failed to encode whistle message");
            }
            break;
        }
    }

    if whistle.detected {
        nao_manager.set_left_ear_led(LeftEar::fill(1.0), Priority::High);
        nao_manager.set_right_ear_led(RightEar::fill(1.0), Priority::High);
    } else {
        nao_manager.set_left_ear_led(LeftEar::fill(0.0), Priority::High);
        nao_manager.set_right_ear_led(RightEar::fill(0.0), Priority::High);
    }
}

fn whistle_preprocessing(
    stft: Arc<Mutex<Stft>>,
    audio_sample: AudioSamplesEvent,
) -> PreprocessingData {
    let preprocess_ear = |sample: &[f32]| {
        let spectrogram = stft
            .lock()
            .unwrap()
            .compute(sample, 0, MEAN_WINDOWS)
            .windows_mean();

        let min_i = MIN_FREQ * spectrogram.powers.len() / NYQUIST;
        let max_i = MAX_FREQ * spectrogram.powers.len() / NYQUIST;

        spectrogram.powers[min_i..=max_i].to_vec()
    };

    PreprocessingData {
        left: preprocess_ear(&audio_sample.left),
        right: preprocess_ear(&audio_sample.right),
    }
}

struct PreprocessingData {
    /// Left ear data.
    left: Vec<f32>,
    /// Right ear data.
    right: Vec<f32>,
}

#[derive(Component)]
struct PreprocessingTask(Task<PreprocessingData>);

fn spawn_whistle_preprocess_task(
    mut commands: Commands,
    detection_state: ResMut<WhistleDetectionState>,
    mut audio_samples: EventReader<AudioSamplesEvent>,
    mut preprocessing_tasks: Query<(&mut PreprocessingTask, Entity)>,
) {
    if preprocessing_tasks.get_single_mut().is_ok() {
        return;
    }

    // Only take the last audio sample to reduce contention in case we are lagging behind
    let Some(audio_sample) = audio_samples.read().last() else {
        return;
    };

    let audio_sample = audio_sample.clone();
    let stft = detection_state.stft.clone();

    let task =
        AsyncComputeTaskPool::get().spawn(async move { whistle_preprocessing(stft, audio_sample) });

    let entity = commands.spawn_empty().id();
    commands.entity(entity).insert(PreprocessingTask(task));
}

fn despawn_whistle_preprocessing_task(
    mut commands: Commands,
    mut preprocessing_tasks: Query<(&mut PreprocessingTask, Entity)>,
) {
    let Ok((_, entity)) = &mut preprocessing_tasks.get_single_mut() else {
        return;
    };

    commands.entity(*entity).despawn();
}

fn spawn_whistle_detection_model(
    mut commands: Commands,
    mut model: ResMut<ModelExecutor<WhistleDetectionModel>>,
    mut preprocessing_tasks: Query<&mut PreprocessingTask>,
) {
    let Ok(preprocessing_task) = &mut preprocessing_tasks.get_single_mut() else {
        return;
    };

    if !preprocessing_task.0.is_finished() {
        return;
    }

    let Some(model_input) = block_on(future::poll_once(&mut preprocessing_task.0)) else {
        return;
    };

    commands
        .infer_model(&mut model)
        .with_batched_input(&[&model_input.left, &model_input.right])
        .create_entities()
        .spawn(|detections| Some(WhistleDetections { detections }));
}
