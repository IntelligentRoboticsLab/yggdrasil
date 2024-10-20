//! TODO: We should migrate this module to either cpal or bevy's audio implementation!

use crate::prelude::*;
use bevy::prelude::*;

use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};
use miette::{Context, IntoDiagnostic};
use std::sync::{Arc, Mutex};

use super::battery_sound::BatterySoundPlugin;
use super::wee_sound::WeeSoundPlugin;

const VOLUME_ENV_VARIABLE_NAME: &str = "YGGDRASIL_VOLUME";

/// A sound which can be played by the [`SoundManager`].
///
/// These sounds are streamed into memory on demand.

// When adding new sounds the path should be specified in [`Sound::file_path`].
pub enum Sound {
    Weee,
    Ghast,
    ChargeMe,
}

impl Sound {
    fn file_path(&self) -> &'static str {
        match self {
            Self::Weee => "./assets/sounds/weeeee.wav",
            Self::Ghast => "./assets/sounds/ghast.wav",
            Self::ChargeMe => "./assets/sounds/batterysound.wav",
        }
    }
}

/// Module to add the [`SoundManager`] as a resource to the framework.
pub struct SoundManagerPlugin;

impl Plugin for SoundManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SoundManager>()
            .add_plugins(WeeSoundPlugin)
            .add_plugins(BatterySoundPlugin);
    }
}

#[derive(Resource)]
/// A threadsafe `SoundManager` to handle loading and playing sounds.
pub struct SoundManager {
    audio_manager: Arc<Mutex<AudioManager<DefaultBackend>>>,
    volume: f64,
}

impl SoundManager {
    /// Plays a sound using a name from enum Sound.
    pub fn play_sound(&self, sound: Sound) -> Result<()> {
        let mut audio_manager = self.audio_manager.lock().unwrap();
        let streaming_sound = StreamingSoundData::from_file(
            sound.file_path(),
            StreamingSoundSettings::new().volume(self.volume),
        )
        .into_diagnostic()
        .with_context(|| format!("Failed to load sound file: {}", sound.file_path()))?;

        audio_manager.play(streaming_sound).into_diagnostic()?;
        Ok(())
    }
}

impl Default for SoundManager {
    fn default() -> Self {
        let audio_manager = AudioManager::new(AudioManagerSettings::default()).unwrap();
        let volume_string = std::env::var(VOLUME_ENV_VARIABLE_NAME).unwrap_or_else(|_| {
            panic!("Failed to read environment variable `{VOLUME_ENV_VARIABLE_NAME}`")
        });
        let volume: f64 = volume_string.parse().unwrap();

        SoundManager {
            audio_manager: Arc::new(Mutex::new(audio_manager)),
            volume,
        }
    }
}
