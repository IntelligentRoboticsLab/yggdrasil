use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use color_eyre::Result;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};
use tyr::prelude::*;

#[derive(PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Sounds {
    WeeSound,
}

pub struct SoundManagerModule;

impl Module for SoundManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(initialize_sounds)
    }
}

// Init all sounds on startup
fn initialize_sounds(storage: &mut Storage) -> Result<()> {
    let mut sound_manager = SoundManager::new()?;
    sound_manager.load_sound(Sounds::WeeSound, "sounds/weeeee.wav")?;

    storage.add_resource(Resource::new(sound_manager))
}

pub struct SoundManager {
    audio_manager: Arc<Mutex<AudioManager<DefaultBackend>>>,
    mapping: std::collections::HashMap<Sounds, StaticSoundData>,
}

impl SoundManager {
    pub fn new() -> Result<Self> {
        let audio_manager = AudioManager::new(AudioManagerSettings::default())?;
        Ok(SoundManager {
            audio_manager: Arc::new(Mutex::new(audio_manager)),
            mapping: HashMap::new(),
        })
    }

    pub fn load_sound(&mut self, sound: Sounds, file_path: &str) -> Result<()> {
        self.mapping.insert(
            sound,
            StaticSoundData::from_file(file_path, StaticSoundSettings::default())?,
        );

        Ok(())
    }

    pub fn play_sound(&mut self, sound: Sounds) -> Result<()> {
        let mut audio_manager = self.audio_manager.lock().unwrap();
        audio_manager.play(self.mapping.get(&sound).unwrap().clone())?;

        Ok(())
    }
}
