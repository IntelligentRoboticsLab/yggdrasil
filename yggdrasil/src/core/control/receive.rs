use bevy::prelude::*;
use heimdall::CameraPosition;
use re_control_comms::{
    app::NotifyConnection,
    debug_system::DebugEnabledSystems,
    protocol::{
        ViewerMessage, control::ViewerControlMessage, game_controller::ViewerGameControllerMessage,
    },
};

use futures::channel::mpsc::UnboundedReceiver;

use crate::{
    game_controller::GameControllerMessageEvent,
    vision::{
        camera::CameraConfig, referee::recognize::RecognizeRefereePose, scan_lines::ScanLinesConfig,
    },
};

pub(super) struct ControlReceivePlugin;

impl Plugin for ControlReceivePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DebugEnabledSystemUpdated>()
            .add_event::<ViewerConnected>()
            .add_event::<ViewerControlMessageEvent>()
            .add_event::<ViewerGameControllerMessageEvent>()
            .add_systems(
                Update,
                handle_notify_on_connection.run_if(resource_exists::<ViewerMessageReceiver>),
            )
            .add_systems(
                Update,
                (
                    handle_viewer_message,
                    (
                        handle_viewer_control_message,
                        handle_viewer_game_controller_message,
                    ),
                )
                    .chain()
                    .run_if(resource_exists::<ViewerMessageReceiver>),
            );
    }
}

#[derive(Event)]
pub(crate) struct ViewerConnected;

#[derive(Event)]
pub(crate) struct DebugEnabledSystemUpdated;

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
fn handle_viewer_message(
    mut message_receiver: ResMut<ViewerMessageReceiver>,
    mut control_message: EventWriter<ViewerControlMessageEvent>,
    mut game_controller_message: EventWriter<ViewerGameControllerMessageEvent>,
) {
    while let Some(message) = message_receiver.try_recv() {
        match message {
            ViewerMessage::ViewerControlMessage(viewer_control_message) => {
                control_message.write(ViewerControlMessageEvent(viewer_control_message));
            }
            ViewerMessage::ViewerGameController(viewer_game_controller_message) => {
                game_controller_message.write(ViewerGameControllerMessageEvent(
                    viewer_game_controller_message,
                ));
            }
        }
    }
}

#[derive(Event)]
pub(super) struct ViewerControlMessageEvent(ViewerControlMessage);

#[derive(Event)]
pub(super) struct ViewerGameControllerMessageEvent(ViewerGameControllerMessage);

pub(super) fn handle_viewer_control_message(
    mut message_event: EventReader<ViewerControlMessageEvent>,
    mut debug_enabled_systems: ResMut<DebugEnabledSystems>,
    mut ev_debug_enabled_system_updated: EventWriter<DebugEnabledSystemUpdated>,
    mut camera_config: ResMut<CameraConfig>,
    mut scan_lines_config: ResMut<ScanLinesConfig>,
    mut recognize_pose: EventWriter<RecognizeRefereePose>,
) {
    for message in message_event.read() {
        let message = &message.0;
        match message {
            ViewerControlMessage::UpdateEnabledDebugSystem {
                system_name,
                enabled,
            } => {
                debug_enabled_systems.set_system(system_name.clone(), *enabled);
                ev_debug_enabled_system_updated.write(DebugEnabledSystemUpdated);
            }
            ViewerControlMessage::CameraExtrinsic {
                camera_position,
                extrinsic_rotation: rotation,
            } => {
                let config = match camera_position {
                    CameraPosition::Top => &mut camera_config.top,
                    CameraPosition::Bottom => &mut camera_config.bottom,
                };

                config.calibration.extrinsic_rotation = *rotation;
            }
            ViewerControlMessage::FieldColor { config } => {
                *scan_lines_config = config.clone().into();
            }
            ViewerControlMessage::VisualRefereeRecognition => {
                recognize_pose.write(RecognizeRefereePose);
            }
            _ => tracing::warn!(?message, "unhandled message"),
        }
    }
}

fn handle_viewer_game_controller_message(
    mut message_event: EventReader<ViewerGameControllerMessageEvent>,
    mut game_controller_message_sender: EventWriter<GameControllerMessageEvent>,
) {
    for message in message_event.read() {
        let message = &message.0;
        match message {
            ViewerGameControllerMessage::GameControllerMessage { message } => {
                game_controller_message_sender.write(GameControllerMessageEvent(*message));
            }
        }
    }
}

// System that keeps watch on new client connections
pub(super) fn handle_notify_on_connection(
    mut notify_connection_receiver: ResMut<NotifyConnectionReceiver>,
    mut ev_viewer_connected: EventWriter<ViewerConnected>,
) {
    while notify_connection_receiver.try_recv().is_some() {
        ev_viewer_connected.write(ViewerConnected);
    }
}
