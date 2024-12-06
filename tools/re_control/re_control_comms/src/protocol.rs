use std::{collections::HashMap, fmt::Debug};

use bifrost::serialization::{Decode, Encode};
use heimdall::CameraPosition;
use nalgebra::Vector3;

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 1337;

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Resources(HashMap<String, String>),
    DebugEnabledSystems(HashMap<String, bool>),
    CameraExtrinsic {
        camera_position: CameraPosition,
        extrinsic_rotation: Vector3<f32>,
    }
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
    }
}

// #[derive(Encode, Decode, Debug, Clone, PartialEq)]
// pub enum ExtrinsicRotation {
//     Pitch(f32),
//     Roll(f32),
//     Yaw(f32),
// }
