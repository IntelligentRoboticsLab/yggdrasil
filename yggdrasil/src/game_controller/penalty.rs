use std::time::{Duration, Instant};

use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, Penalty};

use crate::core::config::showtime::PlayerConfig;

use super::receive::handle_messages;

/// Plugin responsible for tracking the penalized state of the robot. With the [`PenaltyState`] resource,
/// you can check if the robot is penalized, the type of penalty, and if it just entered or left a penalty.
pub struct PenaltyStatePlugin;

impl Plugin for PenaltyStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PenaltyState>()
            .add_systems(PreUpdate, update_penalty_state.after(handle_messages));
    }
}

/// Returns true if the robot became unpenalized less than the given duration ago
pub fn elapsed_since_penalty_return_less_than(
    duration: Duration,
) -> impl Fn(Res<PenaltyState>) -> bool {
    move |penalty: Res<PenaltyState>| {
        matches!(penalty.current, Penalty::None) && penalty.duration_since_return() < duration
    }
}

/// Returns true if the robot is currently penalized
#[must_use]
pub fn is_penalized(penalty: Res<PenaltyState>) -> bool {
    penalty.is_penalized()
}

fn get_penalty(
    gcm: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
) -> Penalty {
    gcm.and_then(|gcm| {
        gcm.team(player_config.team_number)
            .map(|team| team.players[player_config.player_number as usize - 1].penalty)
    })
    .unwrap_or(Penalty::None)
}

fn update_penalty_state(
    mut penalty: ResMut<PenaltyState>,
    gcm: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
) {
    penalty.previous = penalty.current;
    penalty.current = get_penalty(gcm, player_config);

    if penalty.left_penalty() {
        penalty.last_return = Some(Instant::now());
    }
}

/// Tracks the state of the current and previous penalty
#[derive(Resource, Debug, Clone, Copy)]
pub struct PenaltyState {
    previous: Penalty,
    current: Penalty,
    last_return: Option<Instant>,
}

impl Default for PenaltyState {
    fn default() -> Self {
        Self {
            previous: Penalty::None,
            current: Penalty::None,
            last_return: None,
        }
    }
}

impl PenaltyState {
    #[must_use]
    /// Returns the current game controller [`Penalty`] state.
    ///
    /// # Warning ⚠️
    ///
    /// This currently also has a None variant
    pub fn current(&self) -> Penalty {
        self.current
    }

    /// Returns true if the robot is currently penalized
    #[must_use]
    pub fn is_penalized(&self) -> bool {
        !matches!(self.current, Penalty::None)
    }

    /// Returns true if the robot just entered a penalty
    #[must_use]
    pub fn entered_penalty(&self) -> bool {
        matches!(self.previous, Penalty::None) && !matches!(self.current, Penalty::None)
    }

    /// Returns true if the robot just left a penalty
    #[must_use]
    pub fn left_penalty(&self) -> bool {
        !matches!(self.previous, Penalty::None) && matches!(self.current, Penalty::None)
    }

    /// Duration since the robot has returned from its last penalty
    #[must_use]
    pub fn duration_since_return(&self) -> Duration {
        self.last_return.map_or(Duration::MAX, |last_return| {
            Instant::now().duration_since(last_return)
        })
    }
}
