use crate::prelude::*;

use super::sound_manager::{Sound, SoundManager};

use nidhogg::NaoState;

pub struct BatterySoundModule;

impl Module for BatterySoundModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(battery_sound_system)
            .add_resource(Resource::new(BatteryInfo::default()))
    }
}

#[derive(Default)]
pub struct BatteryInfo {
    prev_battery_level: Option<u32>,
}

#[system]
pub fn battery_sound_system(
    nao_state: &mut NaoState,
    battery_info: &mut BatteryInfo,
    sounds: &mut SoundManager,
) -> Result<()> {

    let battery_level = (nao_state.battery.charge * 100.0) as u32;

    // Check whether previous battery level is initialized
    if let Some(prev_battery_level) = battery_info.prev_battery_level {

        if prev_battery_level == 11 && battery_level <= 10 {
            sounds.play_sound(Sound::ChargeMe)?;
        }
        else if prev_battery_level == 6 && battery_level <= 5 {
            sounds.play_sound(Sound::ChargeMe)?;
        }
    }
    else {

        if battery_level <= 10 {
            sounds.play_sound(Sound::ChargeMe)?;
        }
        else if battery_level <= 5 {
            sounds.play_sound(Sound::ChargeMe)?;
        }
    }

    battery_info.prev_battery_level = Some(battery_level);

    Ok(())
}


