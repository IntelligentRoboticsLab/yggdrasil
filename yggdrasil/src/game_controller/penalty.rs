use std::time::{Duration, Instant};

use bevy::prelude::*;
use bifrost::communication::GameControllerMessage;

use crate::core::config::showtime::PlayerConfig;

use super::receive::handle_messages;

pub struct PenaltyStatePlugin;

impl Plugin for PenaltyStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_penalty_state.after(handle_messages));
    }
}

pub fn elapsed_since_penalty_return_less_than(
    duration: Duration,
) -> impl Fn(Res<PenaltyState>) -> bool {
    move |penalty: Res<PenaltyState>| penalty.duration_since_return() < duration
}

pub fn is_penalized(gcm: Option<&GameControllerMessage>, player_config: &PlayerConfig) -> bool {
    gcm.is_some_and(|gcm| {
        gcm.team(player_config.team_number)
            .is_some_and(|team| team.is_penalized(player_config.player_number))
    })
}

fn update_penalty_state(
    mut penalty: ResMut<PenaltyState>,
    gcm: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
) {
    penalty.previous = penalty.current;
    penalty.current = is_penalized(gcm.as_ref().as_deref(), &player_config);

    if penalty.previous && !penalty.current {
        penalty.last_return = Some(Instant::now());
    }
}

/// Tracks the state of the current and previous penalty
#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct PenaltyState {
    previous: bool,
    current: bool,
    last_return: Option<Instant>,
}

impl PenaltyState {
    /// Duration since the robot has returned from its last penalty
    pub fn duration_since_return(&self) -> Duration {
        self.last_return.map_or(Duration::MAX, |last_return| {
            Instant::now().duration_since(last_return)
        })
    }
}
