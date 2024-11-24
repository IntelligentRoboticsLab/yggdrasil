use std::{collections::HashMap, fmt::Debug};

use bifrost::serialization::{Decode, Encode};

use crate::debug_system::DebugEnabledSystems;

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 40001;

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Disconnect,
    Resources(HashMap<String, String>),
    DebugEnabledSystems(DebugEnabledSystems),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerMessage {
    Disconnect,
    UpdateResource(String, String),
    SendResourcesNow,
    UpdateEnabledDebugSystem(String, bool),
}
