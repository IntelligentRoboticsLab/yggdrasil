use crate::prelude::*;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::streaming::{StreamingSoundData, StreamingSoundSettings},
};
use miette::{Context, IntoDiagnostic};
use std::sync::{Arc, Mutex};

/// A sound which can be played by the [`SoundManager`].
///
/// These sounds are streamed into memory on demand.

// When adding new sounds the path should be specified in [`Sound::file_path`].
pub enum Sound {
    Weee,
    Ghast,
}

impl Sound {
    fn file_path(&self) -> &'static str {
        match self {
            Self::Weee => "./assets/sounds/weeeee.wav",
            Self::Ghast => "./assets/sounds/ghast.wav",
        }
    }
}

/// Module to add the [`SoundManager`] as a resource to the framework.
pub struct SoundManagerModule;

impl Module for SoundManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_resource(Resource::new(SoundManager::new()?))
    }
}

/// A threadsafe SoundManager to handle loading and playing sounds.
pub struct SoundManager {
    audio_manager: Arc<Mutex<AudioManager<DefaultBackend>>>,
}

impl SoundManager {
    /// Creates a new AudioManager with default settings.
    pub fn new() -> Result<Self> {
        let audio_manager = AudioManager::new(AudioManagerSettings::default()).into_diagnostic()?;
        Ok(SoundManager {
            audio_manager: Arc::new(Mutex::new(audio_manager)),
        })
    }

    /// Plays a sound using a name from enum Sound.
    pub fn play_sound(&self, sound: Sound) -> Result<()> {
        let mut audio_manager = self.audio_manager.lock().unwrap();
        let streaming_sound = StreamingSoundData::from_file(
            sound.file_path(),
            StreamingSoundSettings::new().volume(0.1),
        )
        .into_diagnostic()
        .with_context(|| format!("Failed to load sound file: {}", sound.file_path()))?;

        audio_manager.play(streaming_sound).into_diagnostic()?;
        Ok(())
    }
}
