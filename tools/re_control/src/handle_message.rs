use std::sync::{Arc, RwLock};

use re_control_comms::{debug_system::DebugEnabledSystems, protocol::RobotMessage};

use crate::control::ControlStates;

pub fn handle_message(message: &RobotMessage, states: Arc<RwLock<ControlStates>>) {
    match message {
        RobotMessage::DebugEnabledSystems(enabled_systems) => {
            states
                .write()
                .expect("Failed to lock states")
                .debug_enabled_systems_view
                .update(DebugEnabledSystems::from_map(enabled_systems.clone()));
        }
        RobotMessage::Resources(_resources) => {
            tracing::warn!("Got a resource update but is unhandled")
        }
    }
}
