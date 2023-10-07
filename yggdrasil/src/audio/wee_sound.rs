use miette::Result;
use std::time::{Duration, Instant};
use tyr::prelude::*;

use crate::filter::fsr::Contacts;

use super::sound_manager::{Sound, SoundManager};

const WEE_SOUND_TIMEOUT: Duration = Duration::from_secs(3);

/// Add the [`WeeSound`] as a resource, and [`wee_sound_system`] as a system to the framework.
pub struct WeeSoundModule;

impl Module for WeeSoundModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(wee_sound_system)
            .add_resource(Resource::new(WeeSound::default()))
    }
}

/// WeeSound componenet to play a sound with a timeout.
#[derive(Default)]
pub struct WeeSound {
    sound_played: bool,
    last_played: Option<Instant>,
}

impl WeeSound {
    fn timed_out(&self) -> bool {
        matches!(self.last_played, Some(instant) if instant.elapsed() < WEE_SOUND_TIMEOUT)
    }
}

#[system]
pub fn wee_sound_system(
    wee_sound: &mut WeeSound,
    sounds: &mut SoundManager,
    contacts: &Contacts,
) -> Result<()> {
    if wee_sound.timed_out() {
        return Ok(());
    }

    // Play the sound once upon losing ground contact
    if contacts.ground && !wee_sound.sound_played {
        wee_sound.sound_played = true;
        wee_sound.last_played = Some(Instant::now());
        sounds.play_sound(Sound::Weee)?;
    // Reset played state upon regaining ground contact
    } else if !contacts.ground {
        wee_sound.sound_played = false;
    }

    Ok(())
}
