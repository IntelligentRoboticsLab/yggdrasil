//! Contains the dimensions of the robot in meters.
//!
//! The origin is the center of the robot's torso, the x-axis points forward, the y-axis points left,
//! and the z-axis points up.
use nalgebra::{vector, Vector3};

/// Vector pointing from torso to robot frame.
pub const ROBOT_TO_TORSO: Vector3<f32> = vector![-0.0413, 0.0, -0.12842];
/// Vector pointing from robot frame to neck.
pub const ROBOT_TO_NECK: Vector3<f32> = vector![0.0, 0.0, 0.2115];
/// Vector pointing from robot frame to the left pelvis.
pub const ROBOT_TO_LEFT_PELVIS: Vector3<f32> = vector![0.0, 0.05, 0.0];
/// Vector pointing from robot frame to the right pelvis.
pub const ROBOT_TO_RIGHT_PELVIS: Vector3<f32> = vector![0.0, -0.05, 0.0];
/// Vector pointing from the hip to the knee (identical for both legs).
pub const HIP_TO_KNEE: Vector3<f32> = vector![0.0, 0.0, -0.1];
/// Vector pointing from the knee to the ankle (identical for both legs).
pub const KNEE_TO_ANKLE: Vector3<f32> = vector![0.0, 0.0, -0.1029];
/// Vector pointing from the ankle to the sole (identical for both legs).
pub const ANKLE_TO_SOLE: Vector3<f32> = vector![0.0, 0.0, -0.04519];
/// Vector pointing from the robot frame to the left shoulder.
pub const ROBOT_TO_LEFT_SHOULDER: Vector3<f32> = vector![0.0, 0.098, 0.185];
/// Vector pointing from the robot frame to the right shoulder.
pub const ROBOT_TO_RIGHT_SHOULDER: Vector3<f32> = vector![0.0, -0.098, 0.185];
/// Vector pointing from the left shoulder to the left elbow.
pub const LEFT_SHOULDER_TO_LEFT_ELBOW: Vector3<f32> = vector![0.105, 0.015, 0.0];
/// Vector pointing from the right shoulder to the right elbow.
pub const RIGHT_SHOULDER_TO_RIGHT_ELBOW: Vector3<f32> = vector![0.105, -0.015, 0.0];
/// Vector pointing from the elbow to the wrist (identical for both arms).
pub const ELBOW_TO_WRIST: Vector3<f32> = vector![0.05595, 0.0, 0.0];
/// Vector pointing from the neck to the top camera.
pub const NECK_TO_TOP_CAMERA: Vector3<f32> = vector![0.05871, 0.0, 0.06364];
/// Vector pointing from the neck to the bottom camera.
pub const NECK_TO_BOTTOM_CAMERA: Vector3<f32> = vector![0.05071, 0.0, 0.01774];
