//! A Rust implementation of [RoboCupGameControlData.h](https://github.com/RoboCup-SPL/GameController/blob/master/examples/c/RoboCupGameControlData.h).
//!
//! # Rules
//! The rules for the Standard Platform League can be found [here](https://spl.robocup.org/wp-content/uploads/SPL-Rules-master.pdf).
//! Rules related to Communication and the `GameController` can be found in section 2.4.2 or from page 11.
//!
//! # Example
//! For an example of how to use this module, see the documentation for the `SPLStandardMessage` struct.
//!

use crate::serialization::{Decode, Encode};

/// The port from which the `GameController` sends the [`GameControllerMessage`] to the robots.
pub const GAME_CONTROLLER_DATA_PORT: u16 = 3838;

/// The port on which the robots send the [`GameControllerReturnMessage`] data to the `GameController`.
pub const GAME_CONTROLLER_RETURN_PORT: u16 = 3939;

/// The header of the data sent by the `GameController`.
const GAME_CONTROLLER_STRUCT_HEADER: [u8; 4] = [b'R', b'G', b'm', b'e'];

/// The version of the data sent by the `GameController`.
const GAME_CONTROLLER_STRUCT_VERSION: u8 = 18;

/// The header of the data sent by the robots.
const GAME_CONTROLLER_RETURN_STRUCT_HEADER: [u8; 4] = [b'R', b'G', b'r', b't'];

/// The version of the data sent by the robots.
const GAME_CONTROLLER_RETURN_STRUCT_VERSION: u8 = 4;

/// The maximum number of players
const MAX_NUM_PLAYERS: u8 = 20;

/// Enum for each half of the game.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Half {
    /// First half of the match.
    First = 1,
    /// Second half of the match.
    Second = 0,
}

/// Enum for the team colors.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum TeamColor {
    /// Blue, cyan jersey color.
    Blue = 0,
    /// Red, magenta, pink jersey color.
    Red = 1,
    /// Yellow jersey color.
    Yellow = 2,
    /// Black, dark gray jersey color.
    Black = 3,
    /// White jersey color.
    White = 4,
    /// Green jersey color.
    Green = 5,
    /// Orange jersey color.
    Orange = 6,
    /// Purple, violet jersey color.
    Purple = 7,
    /// Brown jersey color.
    Brown = 8,
    /// Light gray jersey color.
    Gray = 9,
}

/// Enum for the different competition phases.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum CompetitionPhase {
    /// Round-robin phase of the competition.
    RoundRobin = 0,
    /// Playoff phase of the competition.
    PlayOff = 1,
}

/// Enum for the different competition types.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum CompetitionType {
    /// Normal game mode.
    Normal = 0,
    /// Dynamic ball handling game mode (challenge).
    SharedAutonomy = 1,
}

/// Enum for the different game phases.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum GamePhase {
    /// Normal game phase.
    Normal = 0,
    /// Penalty shootout game phase.
    PenaltyShoot = 1,
    /// Overtime game phase.
    Overtime = 2,
    /// Timeout game phase.
    Timeout = 3,
}

/// Enum for the different game states.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum GameState {
    /// Initial game state.
    Initial = 0,
    /// Ready game state.
    Ready = 1,
    /// Set game state
    Set = 2,
    /// Playing game state.
    Playing = 3,
    /// Finished game state.
    Finished = 4,
    /// Standby game state.
    Standby = 5,
}

/// Enum for the different set plays.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum SetPlay {
    /// No set play.
    None = 0,
    /// Goal kick set play.
    GoalKick = 1,
    /// Pushing free kick set play.
    PushingFreeKick = 2,
    /// Corner kick set play.
    CornerKick = 3,
    /// Kick in set play.
    KickIn = 4,
    /// Penalty kick set play.
    PenaltyKick = 5,
}

/// Enum for the different penalty states.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Penalty {
    /// No penalty.
    None = 0,
    /// Ball holding / playing with hands.
    IllegalBallContact = 1,
    /// Pushing opponent.
    PlayerPushing = 2,
    /// Heard whistle too early?
    IllegalMotionInSet = 3,
    /// Fallen, inactive
    InactivePlayer = 4,
    /// Illegal position.
    IllegalPosition = 5,
    /// Left the field
    LeavingTheField = 6,
    /// Requested for pickup.
    RequestForPickup = 7,
    /// Not moving.
    LocalGameStuck = 8,
    /// Illegal position in set
    IllegalPositionInSet = 9,
    /// Illegal stance.
    PlayerStance = 10,
    /// Illegal motion in initial.
    IllegalMotionInStandby = 11,
    /// Penalty for a substitute.
    Substitute = 14,
    /// Penalty for manual override.
    Manual = 15,
}

/// A struct representing the state of each player.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
pub struct RobotInfo {
    /// Penalty state of the player
    pub penalty: Penalty,

    /// Estimate of time till unpenalised
    pub secs_till_unpenalised: u8,
}

impl RobotInfo {
    fn is_penalized(&self) -> bool {
        self.penalty != Penalty::None
    }
}

/// A struct representing the `TeamInfo` of the two teams currently playing.
#[derive(Encode, Decode, Clone, Copy, Debug, PartialEq)]
pub struct TeamInfo {
    /// Unique team number
    pub team_number: u8,

    /// Colour of the field players
    pub field_player_colour: TeamColor,

    /// Colour of the goalkeeper
    pub goalkeeper_colour: TeamColor,

    /// Player number of the goalkeeper (1-MAX_NUM_PLAYERS)
    pub goalkeeper: u8,

    /// Team's score
    pub score: u8,

    /// Penalty shot counter
    pub penalty_shot: u8,

    /// Bits represent penalty shot success
    pub single_shots: u16,

    /// Number of team messages the team is allowed to send for the remainder of the game
    pub message_budget: u16,

    /// The team's players
    pub players: [RobotInfo; MAX_NUM_PLAYERS as usize],
}

impl TeamInfo {
    pub fn is_penalized(&self, player_number: u8) -> bool {
        self.players
            .get(player_number as usize - 1)
            .map(|robot: &RobotInfo| robot.is_penalized())
            .unwrap_or(false)
    }
}

/// A struct representing the `RoboCupGameControlData` received by the Robots.
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct GameControllerMessage {
    /// Header to identify the structure
    pub header: [u8; 4],

    /// Version of the game-controller protocol
    pub version: u8,

    /// Number incremented with each packet sent (with wraparound)
    pub packet_number: u8,

    /// The number of players on a team
    pub players_per_team: u8,

    /// Phase of the competition
    pub competition_phase: CompetitionPhase,

    /// Type of the competition
    pub competition_type: CompetitionType,

    /// Phase of the game
    pub game_phase: GamePhase,

    /// State of the game
    pub state: GameState,

    /// Active set play
    pub set_play: SetPlay,

    /// 1 = game in first half, 0 otherwise
    pub first_half: Half,

    /// The team number of the next team to kick off or free kick
    pub kicking_team: u8,

    /// Estimate of number of seconds remaining in the half
    pub secs_remaining: i16,

    /// Number of seconds shown as secondary time (remaining ready, until free ball, etc)
    pub secondary_time: i16,

    /// Info about the teams
    pub teams: [TeamInfo; 2],
}

impl GameControllerMessage {
    pub fn team(&self, team_number: u8) -> Option<&TeamInfo> {
        self.teams
            .iter()
            .find(|team| team.team_number == team_number)
    }
}

/// A struct representing the `RoboCupGameControlReturnMessage` send by the Robots.
#[derive(Encode, Decode, Debug, PartialEq)]
pub struct GameControllerReturnMessage {
    /// "RGrt"
    pub header: [u8; 4],

    /// Has to be set to GAME_CONTROLLER_RETURN_STRUCT_VERSION
    pub version: u8,

    /// Player number starts with 1
    pub player_num: u8,

    /// Team number
    pub team_num: u8,

    /// 1 means that the robot is fallen, 0 means that the robot can play
    pub fallen: u8,

    /// Position and orientation of the robot
    ///
    /// coordinates in millimeters
    /// 0,0 is in center of field
    /// +ve x-axis points towards the goal we are attempting to score on
    /// +ve y-axis is 90 degrees counter clockwise from the +ve x-axis
    /// angle in radians, 0 along the +x axis, increasing counter clockwise
    pub pose: [f32; 3], // x,y,theta

    /// ball information
    pub ball_age: f32, // seconds since this robot last saw the ball. -1.f if we haven't seen it

    /// Position of ball relative to the robot
    ///
    /// coordinates in millimeters
    /// 0,0 is in center of the robot
    /// +ve x-axis points forward from the robot
    /// +ve y-axis is 90 degrees counter clockwise from the +ve x-axis
    pub ball: [f32; 2],
}

impl GameControllerMessage {
    /// Check if the [`GameControllerMessage`] has a valid header and version and
    /// the number of players does not exceed the maximum number of players per team.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.header == GAME_CONTROLLER_STRUCT_HEADER
            && self.version == GAME_CONTROLLER_STRUCT_VERSION
            && self.players_per_team <= MAX_NUM_PLAYERS
    }
}

/// Implement a new constructor for [`GameControllerReturnMessage`] with default header and version
impl GameControllerReturnMessage {
    /// Construct a new [`GameControllerReturnMessage`] using the specified arguments.
    #[must_use]
    pub fn new(
        player_num: u8,
        team_num: u8,
        fallen: u8,
        pose: [f32; 3],
        ball_age: f32,
        ball: [f32; 2],
    ) -> Self {
        Self {
            header: GAME_CONTROLLER_RETURN_STRUCT_HEADER,
            version: GAME_CONTROLLER_RETURN_STRUCT_VERSION,
            player_num,
            team_num,
            fallen,
            pose,
            ball_age,
            ball,
        }
    }
}
