use crate::behavior::engine::in_behavior;
use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState},
    nao::{NaoManager, Priority},
};

/// Behavior used for preventing damage when the robot is in a falling state.
/// This behavior can only be exited from once the robot is lying down.
///
/// # Notes
/// - Currently, the damage prevention is still quite rudimentary, only
///   unstiffing the joints of the robot and making the head stiff.
///   In future this will be accompanied by an appropriate safe falling
///   position.
/// - If the robot incorrectly assumes it is in a falling state yet
///   will not be lying down, the robot will kinda get "soft-locked".
///   Only by unstiffing the robot will it return to normal.
///   This should not be the case often however, once the falling filter
///   is more advanced.
#[derive(Resource, Copy, Clone, Debug, PartialEq)]
pub struct CatchFall;

impl Behavior for CatchFall {
    const STATE: BehaviorState = BehaviorState::CatchFall;
}

pub struct CatchFallBehaviorPlugin;
impl Plugin for CatchFallBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, catch_fall.run_if(in_behavior::<CatchFall>));
    }
}

pub fn catch_fall(mut nao_manager: ResMut<NaoManager>) {
    nao_manager.unstiff_legs(Priority::Critical);
    nao_manager.unstiff_arms(Priority::Critical);
    nao_manager.unstiff_head(Priority::Critical);
}
