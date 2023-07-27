use color_eyre::Result;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tyr::prelude::*;

#[derive(PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sounds {
    WeeSound,
    GhastSound,
    SheepSound,
}

/// An audio playback manager to play sounds on the Nao.
///
/// This module provides the kira SoundManager as a Resource to the framework.
pub struct SoundManagerModule;

impl Module for SoundManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(initialize_sounds)
    }
}

/// Initialize all sounds on startup.
fn initialize_sounds(storage: &mut Storage) -> Result<()> {
    let mut sound_manager = SoundManager::new()?;
    sound_manager.load_sound(Sounds::WeeSound, "sounds/weeeee.wav")?;
    sound_manager.load_sound(Sounds::GhastSound, "sounds/ghast.wav")?;
    sound_manager.load_sound(Sounds::SheepSound, "sounds/screaming-sheep.wav")?;

    storage.add_resource(Resource::new(sound_manager))
}

/// A threadsafe SoundManager to handle loading and playing sounds.
pub struct SoundManager {
    audio_manager: Arc<Mutex<AudioManager<DefaultBackend>>>,
    mapping: std::collections::HashMap<Sounds, StaticSoundData>,
}

impl SoundManager {
    /// Creates a new AudioManager with default settings.
    pub fn new() -> Result<Self> {
        let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
        Ok(SoundManager {
            audio_manager: Arc::new(Mutex::new(audio_manager)),
            mapping: HashMap::new(),
        })
    }

    /// Loads a sound from a file path and stores it in a hashmap as k: sound, v: StaticSoundData.
    pub fn load_sound(&mut self, sound: Sounds, file_path: &str) -> Result<()> {
        self.mapping.insert(
            sound,
            StaticSoundData::from_file(file_path, StaticSoundSettings::new().volume(0.1))?,
        );

        Ok(())
    }

    /// Plays a sound using a name from enum Sound.
    pub fn play_sound(&mut self, sound: Sounds) -> Result<()> {
        let mut audio_manager = self.audio_manager.lock().unwrap();
        audio_manager.play(self.mapping.get(&sound).unwrap().clone())?;

        Ok(())
    }
}
