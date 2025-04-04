use bevy::prelude::*;

use crate::motion::keyframe::KeyframeExecutor;

use crate::sensor::fsr::GroundContact;

use super::sound_manager::{Sound, SoundManager};
use super::AudioConfig;

use std::time::{Duration, Instant};

const UNGROUNDED_DELAY: Duration = Duration::from_secs(1);

/// Add the [`WeeSound`] as a resource, and [`wee_sound_system`] as a system to the framework.
pub struct WeeSoundPlugin;

impl Plugin for WeeSoundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<WeeSound>()
            .add_systems(Update, wee_sound_system);
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

    fn set_played(&mut self) {
        self.last_played = Some(Instant::now());
        self.sound_played = true;
    }

    fn set_not_played(&mut self) {
        self.sound_played = false;
    }
}

pub fn wee_sound_system(
    mut wee_sound: ResMut<WeeSound>,
    sounds: Res<SoundManager>,
    ground_contact: Res<GroundContact>,
    config: Res<AudioConfig>,
    keyframe_executor: Res<KeyframeExecutor>,
) {
    if wee_sound.timed_out(config.wee_sound_timeout) {
        return;
    }

    if ground_contact
        .ungrounded_since
        .is_some_and(|grounded_since| grounded_since.elapsed() > UNGROUNDED_DELAY)
        && !wee_sound.sound_played
        && !keyframe_executor.is_motion_active()
    {
        wee_sound.set_played();
        sounds
            .play_sound(Sound::Weee)
            .expect("Failed to play wee sound");
    }

    if ground_contact
        .grounded_since
        .is_some_and(|grounded_since| grounded_since.elapsed() > UNGROUNDED_DELAY)
    {
        wee_sound.set_not_played();
    }
}
