use std::collections::HashMap;

use bevy::prelude::*;
use bifrost::serialization::{Decode, Encode};

/// [`DebugEnabledSystems`] keeps track whether a certain system should run,
/// or as the name implies, is enabled.
///
/// [`DebugEnabledSystems`] only keeps the name of a system and not the actual
/// system.
///
/// Since it derives [`Resource`], it can be used in any system to read or
/// update the enabled state of systems.
#[derive(Resource, Encode, Decode, Debug, Default, Clone)]
pub struct DebugEnabledSystems {
    pub systems: HashMap<String, bool>,
}

impl DebugEnabledSystems {
    pub fn set_system(&mut self, system_name: String, enabled: bool) {
        if let Some(current_enabled) = self.systems.get_mut(&system_name) {
            *current_enabled = enabled;
        } else {
            tracing::error!("System `{}` does not exist", system_name);
        }
    }
}

impl From<HashMap<String, bool>> for DebugEnabledSystems {
    fn from(value: HashMap<String, bool>) -> Self {
        DebugEnabledSystems { systems: value }
    }
}
