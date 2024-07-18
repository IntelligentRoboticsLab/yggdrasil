use crate::motion::keyframe::KeyframeExecutor;
use crate::prelude::*;

use crate::sensor::fsr::Contacts;

use super::sound_manager::{Sound, SoundManager};
use super::AudioConfig;

use std::time::{Duration, Instant};

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
    fn timed_out(&self, timeout: Duration) -> bool {
        matches!(self.last_played, Some(instant) if instant.elapsed() < timeout)
    }
}

#[system]
pub fn wee_sound_system(
    wee_sound: &mut WeeSound,
    sounds: &mut SoundManager,
    contacts: &Contacts,
    config: &AudioConfig,
    keyframe_executor: &mut KeyframeExecutor,
) -> Result<()> {
    if wee_sound.timed_out(config.wee_sound_timeout) {
        return Ok(());
    }

    // Play the sound once upon losing ground contact
    if !contacts.ground && !wee_sound.sound_played && !keyframe_executor.is_motion_active() {
        wee_sound.sound_played = true;
        wee_sound.last_played = Some(Instant::now());
        sounds.play_sound(Sound::Weee)?;
    // Reset played state upon regaining ground contact
    } else if contacts.ground {
        wee_sound.sound_played = false;
    }

    Ok(())
}
