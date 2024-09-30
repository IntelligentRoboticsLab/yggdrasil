use bevy::prelude::*;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{InputStreamTimestamp as Timestamp, SampleRate, StreamConfig};

use itertools::Itertools;

/// The amount of samples in a second, typically 44100.
pub const SAMPLE_RATE: SampleRate = SampleRate(44100);
/// How many audio samples to record per channel.
pub const SAMPLES_PER_CHANNEL: u32 = 2048;
/// Record two channels (left/right ear)
pub const CHANNELS: u16 = 2;
pub const TOTAL_SAMPLES: u32 = SAMPLES_PER_CHANNEL * CHANNELS as u32;

/// This module provides the following resources to the application:
/// - [`AudioSample`]
pub struct AudioInputPlugin;

impl Plugin for AudioInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AudioSamplesEvent>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, emit_event)
            .add_systems(Update, log);
    }
}

fn setup(mut commands: Commands) {
    // ALSA is the default host on linux
    // let host = cpal::default_host();
    // Alternatively we can use JACK
    let host = cpal::host_from_id(cpal::HostId::Jack).unwrap();
    tracing::info!("Using audio host `{}`", host.id().name());

    let device = host
        .default_input_device()
        .expect("No input device available.");

    let config = StreamConfig {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = AudioSamples::new(SAMPLE_RATE);

    let stream = device
        .build_input_stream(
            &config,
            {
                let buffer = samples.buffer().clone();
                move |data: &[f32], info| {
                    let mut lock = buffer.lock().unwrap();
                    let AudioBuffer {
                        ref mut last_update,
                        ref mut buffer,
                    } = lock.deref_mut();

                    last_update.insert(info.timestamp());

                    // On local testing, the data buffer is not filled completely by default
                    // TODO: test this on the Nao!
                    //
                    // To force correct sample rate and buffer size with pw-jack
                    // ```sh
                    // pw-metadata -n settings 0 clock.force-rate 44100
                    // pw-metadata -n settings 0 clock.force-quantum 2048
                    // ```
                    let n = data.len().min(buffer.len());
                    println!("Received {} samples", data.len());

                    buffer[..n].copy_from_slice(&data[..n]);
                    buffer[n..].fill(0.0);
                }
            },
            |e| {
                tracing::warn!("Audio input stream error: {e}");
            },
            None,
        )
        .expect("Failed to build input stream");

    stream.play().expect("Failed to play stream");

    // we need to keep the stream around for it to stay alive
    let _ = Box::leak(Box::new(stream));

    commands.insert_resource(samples);
}

type Buffer = [f32; TOTAL_SAMPLES as usize];

pub struct AudioBuffer {
    pub last_update: Option<Timestamp>,
    pub buffer: Buffer,
}

impl AudioBuffer {
    fn new() -> Self {
        Self {
            last_update: None,
            buffer: [0.0; TOTAL_SAMPLES as usize],
        }
    }
}

#[derive(Resource, Clone)]
pub struct AudioSamples {
    rate: SampleRate,
    // interleaved buffer
    buffer: Arc<Mutex<AudioBuffer>>,
}

impl AudioSamples {
    fn new(rate: SampleRate) -> Self {
        Self {
            rate,
            buffer: Arc::new(Mutex::new(AudioBuffer::new())),
        }
    }

    fn rate(&self) -> SampleRate {
        self.rate
    }

    fn last_update(&self) -> Option<Timestamp> {
        self.buffer.lock().unwrap().last_update
    }

    fn buffer(&self) -> &Arc<Mutex<AudioBuffer>> {
        &self.buffer
    }

    fn deinterleave(&self) -> (Vec<f32>, Vec<f32>) {
        let now = std::time::Instant::now();
        let a = self.buffer.lock().unwrap().buffer.iter().tuples().unzip();
        eprintln!("Took {:?}", now.elapsed());
        a
    }
}

#[derive(Event)]
struct AudioSamplesEvent {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

fn emit_event(
    samples: Res<AudioSamples>,
    mut last_timestamp: Local<Option<Timestamp>>,
    mut ev: EventWriter<AudioSamplesEvent>,
) {
    let last_update = samples.last_update();
    if *last_timestamp == last_update {
        return;
    }

    *last_timestamp = last_update;

    let (left, right) = samples.deinterleave();
    ev.send(AudioSamplesEvent { left, right });
}

fn log(mut samples: EventReader<AudioSamplesEvent>) {
    for AudioSamplesEvent { left, right } in samples.read() {
        println!(
            "Left: {:?}..{:?}, Right: {:?}..{:?}",
            &left[..2],
            &left[left.len() - 2..left.len()],
            &right[..2],
            &right[right.len() - 2..right.len()]
        );
    }
}
