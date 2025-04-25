use bifrost::{
    communication::GameControllerMessage,
    serialization::{Decode, Encode},
};

#[derive(Encode, Decode, Debug, Clone, Copy)]
pub struct Player {
    pub player_number: u8,
    pub team_number: u8,
}

/// Possible message that the robot can send to the "game controller" panel
#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotGameController {
    GameControllerMessage { message: GameControllerMessage },
    GameControllerMessageInit { team_number: u8 },
    PlayerInfo { player: Player },
}

/// Possible message that the viewer can send in the "game controller" panel
#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerGameControllerMessage {
    GameControllerMessage { message: GameControllerMessage },
}
