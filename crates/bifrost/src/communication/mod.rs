//! Communication implementation for robot soccer related communication for the Standard Platform League.
mod game_controller_message;

pub use game_controller_message::{
    CompetitionPhase, CompetitionType, GameControllerMessage, GameControllerReturnMessage,
    GamePhase, GameState, Half, Penalty, RobotInfo, SetPlay, TeamColor, TeamInfo,
    GAME_CONTROLLER_DATA_PORT, GAME_CONTROLLER_RETURN_PORT,
};

/// The maximum allowed size in bytes of an udp message for robot-to-robot communication.
pub const ROBOT_TO_ROBOT_MAX_MESSAGE_SIZE: usize = 128;
