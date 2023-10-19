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
    // save pcm
    // call on buffer of n last samples
    audio: Vec<i16>, //placeholder for now
    samples: usize,
}

impl InputAudio {
    pub fn with_samples(mut self, number_samples: usize) -> () {
        self.samples = number_samples;
    }
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

    if input_audio.samples != 0 {
        pcm.start().unwrap();
        let io = pcm.io_i16().unwrap();
        let samples = input_audio.samples;
        let mut buffer = vec![0i16; samples];
        for i in 0..samples {
            let sample = io.readi(&mut buffer[i..i + 1]).unwrap();
            if sample == 0 {
                break;
            }
        }
        input_audio.audio = buffer;
        input_audio.samples = 0;
        println!("{:?}", input_audio.audio);
    }
    Ok(())
}
