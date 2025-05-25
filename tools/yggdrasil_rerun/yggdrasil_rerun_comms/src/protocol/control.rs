use std::collections::HashMap;

use bifrost::serialization::{Decode, Encode};
use heimdall::CameraPosition;
use nalgebra::Vector3;

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

/// Possible message that the robot can send to the "control" panel
#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotControlMessage {
    Resources(HashMap<String, String>),
    DebugEnabledSystems(HashMap<String, bool>),
    CameraExtrinsic {
        camera_position: CameraPosition,
        extrinsic_rotation: Vector3<f32>,
    },
    FieldColor {
        config: FieldColorConfig,
    },
}

/// Possible message that the viewer can send in the "control" panel
#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerControlMessage {
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
}
