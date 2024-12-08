use bevy::prelude::*;
use heimdall::CameraPosition;
use re_control_comms::{
    app::NotifyConnection, debug_system::DebugEnabledSystems, protocol::ViewerMessage,
};

use futures::channel::mpsc::UnboundedReceiver;

use crate::vision::camera::CameraConfig;

use super::{DebugEnabledSystemUpdated, ViewerConnected};

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
    mut camera_config: ResMut<CameraConfig>,
) {
    while let Some(message) = message_receiver.try_recv() {
        #[allow(clippy::single_match_else)]
        match message {
            ViewerMessage::UpdateEnabledDebugSystem {
                system_name,
                enabled,
            } => {
                debug_enabled_systems.set_system(system_name, enabled);
                ev_debug_enabled_system_updated.send(DebugEnabledSystemUpdated);
            }
            ViewerMessage::CameraExtrinsic {
                camera_position,
                extrinsic_rotation: rotation,
            } => {
                let config = match camera_position {
                    CameraPosition::Top => &mut camera_config.top,
                    CameraPosition::Bottom => &mut camera_config.bottom,
                };

                config.calibration.extrinsic_rotation = rotation;
            }
            _ => tracing::warn!(?message, "unhandled message"),
        }
    }
}

// System that keeps watch on new client connections
pub fn handle_notify_on_connection(
    mut notify_connection_receiver: ResMut<NotifyConnectionReceiver>,
    mut ev_viewer_connected: EventWriter<ViewerConnected>,
) {
    while notify_connection_receiver.try_recv().is_some() {
        ev_viewer_connected.send(ViewerConnected);
    }
}
