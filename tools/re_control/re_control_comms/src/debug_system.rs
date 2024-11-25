use std::collections::HashMap;

use bevy::prelude::*;
use bifrost::serialization::{Decode, Encode};

#[derive(Resource, Encode, Decode, Debug, Default, Clone)]
pub struct DebugEnabledSystems {
    pub systems: HashMap<String, bool>,
}

impl DebugEnabledSystems {
    #[must_use]
    pub fn from_map(map: HashMap<String, bool>) -> Self {
        DebugEnabledSystems { systems: map }
    }

    pub fn set_system(&mut self, name: String, enabled: bool) {
        if let Some(current_enabled) = self.systems.get_mut(&name) {
            *current_enabled = enabled;
        } else {
            tracing::error!("System `{}` does not exist", name);
        }
    }
}
