use color_eyre::Result;
use std::time::{Duration, Instant};
use tyr::prelude::*;

use crate::filter::fsr::HasGroundContact;

use super::sound_manager::{SoundManager, Sounds};

const WEE_SOUND_TIMEOUT: Duration = Duration::from_secs(3);

pub struct WeeSoundModule;

impl Module for WeeSoundModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(wee_sound_system)
            .add_resource(Resource::new(WeeSound::default()))
    }
}

#[derive(Default)]
pub struct WeeSound {
    sound_played: bool,
    last_played: Option<Instant>,
}

#[system]
pub fn wee_sound_system(
    play_sound: &mut WeeSound,
    sounds: &mut SoundManager,
    has_ground_contact: &HasGroundContact,
) -> Result<()> {
    if play_sound
        .last_played
        .is_some_and(|last_played| last_played.elapsed() < WEE_SOUND_TIMEOUT)
    {
        return Ok(());
    }

    if !**has_ground_contact && !play_sound.sound_played {
        play_sound.sound_played = true;
        play_sound.last_played = Some(Instant::now());
        sounds.play_sound(Sounds::WeeSound)?;
    } else if **has_ground_contact {
        play_sound.sound_played = false;
    }
    Ok(())
}
