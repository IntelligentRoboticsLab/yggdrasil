use std::{collections::HashMap, fmt::Debug};

use bifrost::{
    communication::GameControllerMessage,
    serialization::{Decode, Encode},
};
use heimdall::CameraPosition;
use nalgebra::Vector3;

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 1337;

#[derive(Encode, Decode, Debug, Clone, Default)]
pub struct FieldColorConfig {
    pub min_edge_luminance_difference: f32,
    /// Field color
    pub max_field_luminance: f32,
    pub min_field_saturation: f32,
    pub min_field_hue: f32,
    pub max_field_hue: f32,
    /// White color
    pub min_white_luminance: f32,
    pub max_white_saturation: f32,
    /// Black color
    pub max_black_luminance: f32,
    pub max_black_saturation: f32,
    /// Green chromaticity threshold
    pub green_chromaticity_threshold: f32,
    pub red_chromaticity_threshold: f32,
    pub blue_chromaticity_threshold: f32,
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Resources(HashMap<String, String>),
    DebugEnabledSystems(HashMap<String, bool>),
    CameraExtrinsic {
        camera_position: CameraPosition,
        extrinsic_rotation: Vector3<f32>,
    },
    FieldColor {
        config: FieldColorConfig,
    },
    RobotGameController(RobotGameController)
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerMessage {
    UpdateResource {
        resource_name: String,
        value: String,
    },
    SendResourcesNow,
    UpdateEnabledDebugSystem {
        system_name: String,
        enabled: bool,
    },
    CameraExtrinsic {
        camera_position: CameraPosition,
        extrinsic_rotation: Vector3<f32>,
    },
    FieldColor {
        config: FieldColorConfig,
    },
    VisualRefereeRecognition,
    ViewerGameController(ViewerGameControllerMessage),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum GameControllerData {
    GameControllerMessage(GameControllerMessage),
    TeamUpdate { team_number: u8 },
}

#[derive(Encode, Decode, Debug, Clone, Copy)]
pub struct Player {
    pub player_number: u8,
    pub team_number: u8,
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotGameController {
    GameControllerMessage { message: GameControllerMessage },
    GameControllerMessageInit { team_number: u8 },
    PlayerInfo {
        player: Player
    }
}


#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerGameControllerMessage {
    GameControllerMessage { message: GameControllerMessage },
}
