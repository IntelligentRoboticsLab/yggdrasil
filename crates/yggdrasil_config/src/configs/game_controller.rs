use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use bevy::prelude::*;

/// Configuration for the game controller.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
#[serde(deny_unknown_fields)]
pub struct GameControllerConfig {
    /// The timeout for the game controller connection.
    ///
    /// If no message is received from the game controller within this time, the connection is considered lost.
    ///
    /// Allows a new game controller to connect after an old one has disconnected.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_timeout: Duration,
    /// The delay between sending return messages to the game controller.
    ///
    /// Used to limit the rate at which the game controller is updated.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_return_delay: Duration,
}
