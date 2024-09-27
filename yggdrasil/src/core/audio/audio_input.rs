use bevy::prelude::*;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, StreamConfig};

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
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands) {
    // ALSA is the default host on linux
    let host = cpal::default_host();
    // Alternatively we can use JACK
    // let host = cpal::host_from_id(cpal::HostId::Jack).unwrap();
    tracing::info!("Using audio host `{}`", host.id().name());

    let device = host
        .default_input_device()
        .expect("No input device available.");

    let config = StreamConfig {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        buffer_size: cpal::BufferSize::Default,
    };

    let samples = AudioSample::new(SAMPLE_RATE);

    let stream = device
        .build_input_stream(
            &config,
            {
                let buffer = samples.buffer().clone();
                move |data: &[f32], _info| {
                    let mut buffer = buffer.lock().unwrap();

                    // On local testing, the data buffer is not filled completely by default
                    // TODO: test this on the Nao!
                    //
                    // To force correct sample rate and buffer size with pw-jack
                    // ```sh
                    // pw-metadata -n settings 0 clock.force-rate 44100
                    // pw-metadata -n settings 0 clock.force-quantum 2048
                    // ```
                    let n = data.len().min(buffer.len());
                    // println!("Received {} samples", data.len());

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

#[derive(Resource)]
pub struct AudioSample {
    rate: SampleRate,
    // interleaved buffer
    buffer: Arc<Mutex<Buffer>>,
}

impl AudioSample {
    pub fn new(rate: SampleRate) -> Self {
        Self {
            rate,
            buffer: Arc::new(Mutex::new([0.0; TOTAL_SAMPLES as usize])),
        }
    }

    pub fn rate(&self) -> SampleRate {
        self.rate
    }

    pub fn buffer(&self) -> &Arc<Mutex<Buffer>> {
        &self.buffer
    }

    pub fn deinterleave(&self) -> (Vec<f32>, Vec<f32>) {
        self.buffer.lock().unwrap().iter().tuples().unzip()
    }
}
