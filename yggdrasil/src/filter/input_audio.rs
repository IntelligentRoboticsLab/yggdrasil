use miette::Result;
use tyr::prelude::*;
use alsa::pcm::*;
use alsa::{Direction, ValueOr};

pub struct InputAudioFilter;

// constants
// sample rate , frames, format, access
const SAMPLE_RATE: u32 = 44100;
const FRAMES: i64 = 2048;
const FORMAT: Format = Format::s16();
const ACCESS: Access = Access::RWInterleaved;

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
    let pcm = PCM::new("default", Direction::Capture, false).unwrap();
    {
        let hwp = HwParams::any(&pcm).unwrap();
        hwp.set_rate(SAMPLE_RATE, ValueOr::Nearest).unwrap();
        hwp.set_format(FORMAT).unwrap();
        hwp.set_access(ACCESS).unwrap();
        hwp.set_period_size(FRAMES, ValueOr::Nearest).unwrap();
        pcm.hw_params(&hwp).unwrap();
    }
    pcm.start().unwrap();
    // input_audio.audio = pcm;
    Ok(())
}
