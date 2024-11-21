use bevy::prelude::*;

use crate::motion::keyframe::KeyframeExecutor;

use crate::sensor::fsr::Contacts;

use super::sound_manager::{Sound, SoundManager};
use super::AudioConfig;

use std::time::{Duration, Instant};

/// Add the [`WeeSound`] as a resource, and [`wee_sound_system`] as a system to the framework.
pub struct WeeSoundPlugin;

impl Plugin for WeeSoundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        // app.init_resource::<WeeSound>()
        //     .add_systems(Update, wee_sound_system);
    }
}

/// `WeeSound` component to play a sound with a timeout.
#[derive(Default, Resource)]
pub struct WeeSound {
    sound_played: bool,
    last_played: Option<Instant>,
}

impl WeeSound {
    fn timed_out(&self, timeout: Duration) -> bool {
        matches!(self.last_played, Some(instant) if instant.elapsed() < timeout)
    }
}

pub fn wee_sound_system(
    mut wee_sound: ResMut<WeeSound>,
    sounds: Res<SoundManager>,
    contacts: Res<Contacts>,
    config: Res<AudioConfig>,
    keyframe_executor: Res<KeyframeExecutor>,
) {
    if wee_sound.timed_out(config.wee_sound_timeout) {
        return;
    }

    // Play the sound once upon losing ground contact
    if !contacts.ground && !wee_sound.sound_played && !keyframe_executor.is_motion_active() {
        wee_sound.sound_played = true;
        wee_sound.last_played = Some(Instant::now());
        sounds
            .play_sound(Sound::Weee)
            .expect("Failed to play wee sound");

    // Reset played state upon regaining ground contact
    } else if contacts.ground {
        wee_sound.sound_played = false;
    }
}
