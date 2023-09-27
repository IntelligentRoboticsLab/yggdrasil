use miette::Result;
use tyr::prelude::*;
use alsa::pcm::*;
use alsa::Direction;

pub struct InputAudioFilter;

// constants
// sample rate , frames, format, access
const SAMPLE_RATE: i32 = 44100;
const FRAMES: i32 = 2048;
const FORMAT: &str = "FloatLE";
const ACCESS: &str = "RWInterleaved";

impl Module for InputAudioFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(input_audio_filter)
            .add_resource(Resource::new(InputAudio::default()))
    }
}

// define InputAudio as vector (number of channels * number of frames)
#[derive(Default)]
pub struct InputAudio{
    audio: Vec<i32>, //placeholder for now
}

#[system]
fn input_audio_filter(
    input_audio: &mut InputAudio) -> Result<()> {
    // record audio, store new audio in struct
    Ok(())
}
