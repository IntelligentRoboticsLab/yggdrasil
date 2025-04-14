use crate::nao::Priority;
use bevy::prelude::*;
use nidhogg::{
    NaoState,
    types::{FillExt, Skull},
};

use super::manager::NaoManager;

const LED_ENABLED: f32 = 1.0;

/// Plugin that adds a battery level display to the robot's skull LEDs.
pub(super) struct BatteryLedPlugin;

impl Plugin for BatteryLedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, battery_display);
    }
}

pub fn battery_display(nao_state: Res<NaoState>, mut manager: ResMut<NaoManager>) {
    let battery_level = (nao_state.battery.charge * 100.0) as u32;

    // turns on a certain amount of LED's based on the robots battery level
    let mut skull = Skull::fill(0.0);
    if battery_level >= 10 {
        skull.left_front_0 = LED_ENABLED;
    }

    if battery_level >= 20 {
        skull.left_front_1 = LED_ENABLED;
    }

    if battery_level >= 30 {
        skull.left_middle_0 = LED_ENABLED;
    }

    if battery_level >= 40 {
        skull.left_rear_0 = LED_ENABLED;
    }

    if battery_level >= 50 {
        skull.left_rear_1 = LED_ENABLED;
        skull.left_rear_2 = LED_ENABLED;
    }

    if battery_level >= 60 {
        skull.right_rear_1 = LED_ENABLED;
        skull.right_rear_2 = LED_ENABLED;
    }

    if battery_level >= 70 {
        skull.right_rear_0 = LED_ENABLED;
    }

    if battery_level >= 80 {
        skull.right_middle_0 = LED_ENABLED;
    }

    if battery_level >= 90 {
        skull.right_front_0 = LED_ENABLED;
    }

    if battery_level >= 100 {
        skull.right_front_1 = LED_ENABLED;
    }

    manager.set_skull_led(skull, Priority::Medium);
}
