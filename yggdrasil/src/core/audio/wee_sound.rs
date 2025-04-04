use bevy::prelude::*;

use crate::motion::keyframe::KeyframeExecutor;

use crate::sensor::falling::FallState;
use crate::sensor::fsr::GroundContact;

use super::sound_manager::{Sound, SoundManager};
use super::AudioConfig;

pub struct WeeSoundPlugin;

impl Plugin for WeeSoundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, wee_sound_system);
    }
}

pub fn wee_sound_system(
    sounds: Res<SoundManager>,
    ground_contact: Res<GroundContact>,
    audio_config: Res<AudioConfig>,
    keyframe_executor: Res<KeyframeExecutor>,
    fall_state: Res<FallState>,
    mut sound_played: Local<bool>,
) {
    if ground_contact
        .ungrounded_since
        .is_some_and(|grounded_since| {
            grounded_since.elapsed() > audio_config.wee_sound_ungrounded_timeout
        })
        && !*sound_played
        && !keyframe_executor.is_motion_active()
        && !matches!(*fall_state, FallState::None)
    {
        sounds
            .play_sound(Sound::Weee)
            .expect("Failed to play wee sound");
        *sound_played = true;
    }

    if ground_contact.grounded_since.is_some_and(|grounded_since| {
        grounded_since.elapsed() > audio_config.wee_sound_grounded_timeout
    }) {
        *sound_played = false;
    }
}
