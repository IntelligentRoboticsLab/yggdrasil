use bevy::prelude::*;
use re_control_comms::{
    app::NotifyConnection, debug_system::DebugEnabledSystems, protocol::ViewerMessage,
};

use futures::channel::mpsc::UnboundedReceiver;

use super::{events::DebugEnabledSystemUpdated, ViewerConnected};

#[derive(Resource)]
pub struct NotifyConnectionReceiver {
    pub rx: UnboundedReceiver<NotifyConnection>,
}

impl NotifyConnectionReceiver {
    pub fn try_recv(&mut self) -> Option<NotifyConnection> {
        self.rx
            .try_next()
            .transpose()
            .expect("Notify on connection message receive channel closed")
            .ok()
    }
}

#[derive(Resource)]
pub struct ViewerMessageReceiver {
    pub rx: UnboundedReceiver<ViewerMessage>,
}

impl ViewerMessageReceiver {
    pub fn try_recv(&mut self) -> Option<ViewerMessage> {
        self.rx
            .try_next()
            .transpose()
            .expect("Control message receive channel closed")
            .ok()
    }
}

// System that handles every messages that are received from every connected
// client
pub fn handle_viewer_message(
    mut message_receiver: ResMut<ViewerMessageReceiver>,
    mut debug_enabled_systems: ResMut<DebugEnabledSystems>,
    mut ev_debug_enabled_system_updated: EventWriter<DebugEnabledSystemUpdated>,
) {
    while let Some(message) = message_receiver.try_recv() {
        #[allow(clippy::single_match_else)]
        match message {
            ViewerMessage::UpdateEnabledDebugSystem(name, enabled) => {
                debug_enabled_systems.set_system(name, enabled);
                ev_debug_enabled_system_updated.send(DebugEnabledSystemUpdated);
            }
            _ => tracing::warn!("Unhandled message"),
        }
    }
}

// System that keeps watch on new client connections
pub fn handle_notify_on_connection(
    mut notify_connection_receiver: ResMut<NotifyConnectionReceiver>,
    mut ev_viewer_connected: EventWriter<ViewerConnected>,
) {
    while let Some(notify_connection) = notify_connection_receiver.try_recv() {
        ev_viewer_connected.send(ViewerConnected(notify_connection.id));
    }
}
