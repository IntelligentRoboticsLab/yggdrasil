use std::collections::HashMap;

use bevy::prelude::Resource;
use bifrost::serialization::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Encode, Decode, Debug, Default, Clone)]
pub struct DebugEnabledSystems {
    pub systems: HashMap<String, bool>,
}

impl DebugEnabledSystems {
    pub fn set_system(&mut self, name: String, enabled: bool) {
        if let Some(current_enabled) = self.systems.get_mut(&name) {
            *current_enabled = enabled;
        } else {
            tracing::error!("System `{}` does not exist", name);
        }
    }
}