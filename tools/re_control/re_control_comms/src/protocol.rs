use std::{collections::HashMap, fmt::Debug};

use bifrost::serialization::{Decode, Encode};

pub type HandlerFn<T> = Box<dyn Fn(&T) + Send + Sync + 'static>;

pub const CONTROL_PORT: u16 = 1337;

#[derive(Encode, Decode, Debug, Clone)]
pub enum RobotMessage {
    Resources(HashMap<String, String>),
    DebugEnabledSystems(HashMap<String, bool>),
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
}
