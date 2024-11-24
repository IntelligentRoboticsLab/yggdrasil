use std::sync::{Arc, RwLock};

use crate::{connection::protocol::RobotMessage, control::ControlStates};

pub fn handle_message(message: &RobotMessage, states: Arc<RwLock<ControlStates>>) {
    match message {
        RobotMessage::Disconnect => {
            tracing::info!("Robot disconnected")
        }
        RobotMessage::DebugEnabledSystems(enabled_systems) => {
            states
                .write()
                .expect("Failed to lock states")
                .debug_enabled_systems_view
                .update(enabled_systems.clone());
        }
        RobotMessage::Resources(_resources) => {
            tracing::warn!("Got a resource update but is unhandled")
        }
    }
}
