use super::AudioConfig;
use super::sound_manager::{Sound, SoundManager};
use bevy::prelude::*;
use nidhogg::NaoState;
use std::time::{Duration, Instant};

const THRESHOLD_LOW: u32 = 10;
const THRESHOLD_CRITICAL: u32 = 5;

pub struct BatterySoundPlugin;

impl Plugin for BatterySoundPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, battery_sound_system)
            .add_systems(PostStartup, init_battery_level);
    }
}

fn init_battery_level(mut battery_info: Local<BatteryInfo>, nao_state: Res<NaoState>) {
    battery_info.prev_level = (nao_state.battery.charge * 100.0) as u32;
}

#[derive(Default, Resource)]
struct BatteryInfo {
    last_played: Option<Instant>,
    prev_level: u32,
}

impl BatteryInfo {
    fn timed_out(&self, timeout: Duration) -> bool {
        matches!(self.last_played, Some(instant) if instant.elapsed() < timeout)
    }

    fn check_level(&mut self, nao_state: &NaoState) -> bool {
        let battery_level = (nao_state.battery.charge * 100.0) as u32;
        let low_or_critical = battery_level == THRESHOLD_LOW || battery_level == THRESHOLD_CRITICAL;

        //  Should play sound at the exact thresholds, or when already below/at threshold at startup
        (battery_level < self.prev_level && low_or_critical)
            || (battery_level <= THRESHOLD_LOW && self.last_played.is_none())
    }
}

fn battery_sound_system(
    mut battery_info: Local<BatteryInfo>,
    sounds: Res<SoundManager>,
    nao_state: Res<NaoState>,
    config: Res<AudioConfig>,
) {
    // Timeout or already charging
    if battery_info.timed_out(config.battery_sound_timeout) || nao_state.battery.status > 0.0 {
        return;
    }

    if battery_info.check_level(&nao_state) {
        sounds
            .play_sound(Sound::ChargeMe)
            .expect("Failed to play battery sound");

        battery_info.last_played = Some(Instant::now());
    }
    battery_info.prev_level = (nao_state.battery.charge * 100.0) as u32;
}
