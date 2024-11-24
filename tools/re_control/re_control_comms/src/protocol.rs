use std::{collections::HashMap, fmt::Debug};

use bifrost::serialization::{Decode, Encode};

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 40001;

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Disconnect,
    Resources(HashMap<String, String>),
    DebugEnabledSystems(HashMap<String, bool>),
}

#[derive(Encode, Decode, Debug, Clone)]
pub enum ViewerMessage {
    Disconnect,
    UpdateResource(String, String),
    SendResourcesNow,
    UpdateEnabledDebugSystem(String, bool),
}
