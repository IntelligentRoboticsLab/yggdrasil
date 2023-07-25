use color_eyre::Result;
use kira::{
    manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings},
    sound::static_sound::{StaticSoundData, StaticSoundSettings},
};
use tyr::prelude::*;

pub struct SoundManagerModule;

impl Module for SoundManagerModule {
    fn initialize(self, app: App) -> Result<App> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())?;
        app.add_resource(Resource::new(SoundManager::new(manager)))
    }
}

pub struct SoundManager {
    audio_manager: AudioManager<DefaultBackend>,
}

impl SoundManager {
    pub fn new(audio_manager: AudioManager<DefaultBackend>) -> Self {
        SoundManager { audio_manager }
    }

    pub fn load_sound(&mut self, file_path: &str) -> Result<StaticSoundData> {
        StaticSoundData::from_file(file_path, StaticSoundSettings::default())
    }

    // pub fn sound_volume(&mut self, sound_data: &mut StaticSoundData, volume: f32) -> Result<()> {
    //     self.audio_manager.set_volume(volume)
    // }

    pub fn play_sound(&mut self, sound_data: &StaticSoundData) -> Result<()> {
        self.audio_manager.play(sound_data.clone())
    }
}
