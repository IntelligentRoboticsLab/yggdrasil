use bevy::prelude::*;
use re_control_comms::{
    app::NotifyConnection, debug_system::DebugEnabledSystems, protocol::ViewerMessage,
};

use futures::channel::mpsc::UnboundedReceiver;

use super::ViewerConnectedEvent;

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

pub fn handle_viewer_message(
    mut message_receiver: ResMut<ViewerMessageReceiver>,
    mut debug_enabled_systems: ResMut<DebugEnabledSystems>,
) {
    while let Some(message) = message_receiver.try_recv() {
        match message {
            ViewerMessage::Disconnect => {
                tracing::info!("Viewer disconnected");
            }
            ViewerMessage::UpdateEnabledDebugSystem(name, enabled) => {
                debug_enabled_systems.set_system(name, enabled);
            }
            _ => tracing::warn!("Unhandled message"),
        }
    }
}

pub fn handle_notify_on_connection(
    mut notify_connection_receiver: ResMut<NotifyConnectionReceiver>,
    mut ev_viewer_connected: EventWriter<ViewerConnectedEvent>,
) {
    while let Some(notify_connection) = notify_connection_receiver.try_recv() {
        ev_viewer_connected.send(ViewerConnectedEvent(notify_connection.id));
    }
}
