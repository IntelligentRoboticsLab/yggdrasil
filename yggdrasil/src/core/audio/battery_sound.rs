use super::sound_manager::{Sound, SoundManager};
use bevy::prelude::*;
use nidhogg::NaoState;

pub struct BatterySoundPlugin;

impl Plugin for BatterySoundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<BatteryInfo>()
            .add_systems(Update, battery_sound_system);
    }
}

#[derive(Default, Resource)]
pub struct BatteryInfo {
    prev_level: Option<u32>,
}

pub fn battery_sound_system(
    mut battery_info: ResMut<BatteryInfo>,
    sounds: Res<SoundManager>,
    nao_state: Res<NaoState>,
) {
    let mut should_play_sound = false;
    // Integer comparison (prevents warning)
    let battery_level = (nao_state.battery.charge * 100.0) as u32;

    // Check whether previous battery level is initialized
    if let Some(prev_level) = battery_info.prev_level {
        if (prev_level == 10 && battery_level <= 9) || (prev_level == 6 && battery_level <= 5) {
            should_play_sound = true;
        }
    } else if battery_level <= 10 {
        should_play_sound = true;
    }

    if should_play_sound {
        sounds
            .play_sound(Sound::ChargeMe)
            .expect("Failed to play battery sound");
    }
    battery_info.prev_level = Some(battery_level);
}
