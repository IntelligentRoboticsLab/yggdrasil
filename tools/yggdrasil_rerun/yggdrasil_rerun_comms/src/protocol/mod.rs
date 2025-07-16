//! This module provides the the protocol used between robot (host) and viewer
//! (client) to send messages.
//!
//! The protocol is split in messages that the robot sends ([`RobotMessage`])
//! and messages that the viewer sends ([`ViewerMessage`])

pub mod control;
pub mod game_controller;

use std::fmt::Debug;

use bifrost::serialization::{Decode, Encode};
use control::{RobotControlMessage, ViewerControlMessage};
use game_controller::{RobotGameController, ViewerGameControllerMessage};

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 1337;

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    RobotControlMessage(RobotControlMessage),
    RobotGameController(RobotGameController),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerMessage {
    ViewerControlMessage(ViewerControlMessage),
    ViewerGameController(ViewerGameControllerMessage),
}
