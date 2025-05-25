use std::{collections::HashMap, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*, tasks::IoTaskPool};
use bifrost::communication::GameControllerMessage;
use heimdall::CameraPosition;
use yggdrasil_rerun_comms::{
    app::ControlAppHandle,
    debug_system::DebugEnabledSystems,
    protocol::{
        RobotMessage,
        control::RobotControlMessage,
        game_controller::{Player, RobotGameController},
    },
};

use crate::{
    core::config::showtime::PlayerConfig,
    vision::{camera::CameraConfig, scan_lines::ScanLinesConfig},
};

use super::{
    handle_notify_on_connection, handle_viewer_control_message,
    receive::{DebugEnabledSystemUpdated, ViewerConnected},
};

pub(super) struct ControlTransmitPlugin;

impl Plugin for ControlTransmitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SendRobotInitialDataSystems>()
            .add_systems(
                Update,
                send_on_connection.after(handle_notify_on_connection),
            )
            .add_systems(
                Update,
                update_debug_systems_for_clients
                    .after(handle_viewer_control_message)
                    .run_if(resource_exists::<ControlAppHandle>),
            );
    }
}

const SEND_STATE_DELAY: Duration = Duration::from_millis(2_000);

#[derive(Deref, DerefMut)]
pub struct ControlRobotMessageDelay(Timer);

impl Default for ControlRobotMessageDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

/// Send data when a new viewer/client connects to the robot. The data is
/// retrieved and send by all one-shot-systems in the resource
/// [`SendRobotInitialDataSystems`].
///
/// **Note**: In reality all data that is send, is broadcasted to all connected
/// viewers.
pub(crate) fn send_on_connection(
    mut commands: Commands,
    mut viewer_events: EventReader<ViewerConnected>,
    send_robot_initial_data_systems: Res<SendRobotInitialDataSystems>,
) {
    if !viewer_events.is_empty() {
        viewer_events.clear();

        for system_id in &send_robot_initial_data_systems.system_ids {
            commands.run_system(*system_id);
        }
    }
}

// Sends the current state of `DebugEnabledSystems` to the client that
// connected.
pub fn send_debug_enabled_systems(
    debug_enabled_resources: Res<DebugEnabledSystems>,
    control_handle: Res<ControlAppHandle>,
) {
    let msg = RobotMessage::RobotControlMessage(RobotControlMessage::DebugEnabledSystems(
        debug_enabled_resources.systems.clone(),
    ));

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send DebugEnabledSystems");
        }
    })
    .detach();
}

pub fn send_camera_extrinsic(
    camera_config: Res<CameraConfig>,
    control_handle: Res<ControlAppHandle>,
) {
    let top_extrinsic = camera_config.top.calibration.extrinsic_rotation;
    let bottom_extrinsic = camera_config.bottom.calibration.extrinsic_rotation;

    let top_msg = RobotMessage::RobotControlMessage(RobotControlMessage::CameraExtrinsic {
        camera_position: CameraPosition::Top,
        extrinsic_rotation: top_extrinsic,
    });

    let bot_msg = RobotMessage::RobotControlMessage(RobotControlMessage::CameraExtrinsic {
        camera_position: CameraPosition::Bottom,
        extrinsic_rotation: bottom_extrinsic,
    });

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        for msg in [bot_msg, top_msg] {
            if let Err(error) = handle.broadcast(msg).await {
                tracing::error!(?error, "Failed to send camera extrinsic rotation");
            }
        }
    })
    .detach();
}

fn send_green_chromaticity_threshold(
    scan_lines_config: Res<ScanLinesConfig>,
    control_handle: Res<ControlAppHandle>,
) {
    let msg = RobotMessage::RobotControlMessage(RobotControlMessage::FieldColor {
        config: scan_lines_config.as_ref().into(),
    });

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send green chromaticity threshold");
        }
    })
    .detach();
}

fn send_game_controller_player(
    control_handle: Res<ControlAppHandle>,
    player_config: Res<PlayerConfig>,
) {
    let msg = RobotMessage::RobotGameController(RobotGameController::PlayerInfo {
        player: Player {
            player_number: player_config.player_number,
            team_number: player_config.team_number,
        },
    });

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send game controller message");
        }
    })
    .detach();
}

fn send_game_controller_message(
    control_handle: Res<ControlAppHandle>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    player_config: Res<PlayerConfig>,
) {
    let msg = {
        if let Some(message) = game_controller_message {
            RobotMessage::RobotGameController(RobotGameController::GameControllerMessage {
                message: *message,
            })
        } else {
            RobotMessage::RobotGameController(RobotGameController::GameControllerMessageInit {
                team_number: player_config.team_number,
            })
        }
    };

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send game controller message");
        }
    })
    .detach();
}

// This system sends the current [`DebugEnabledSystems`] to all connected
// clients.
// When an individual client updates a debug enabled system, the state of
// other connected clients should also be updated.
pub(crate) fn update_debug_systems_for_clients(
    debug_enabled_resources: Res<DebugEnabledSystems>,
    control_handle: Res<ControlAppHandle>,
    mut ev_debug_enabled_system_updated: EventReader<DebugEnabledSystemUpdated>,
) {
    for _ev in ev_debug_enabled_system_updated.read() {
        let msg = RobotMessage::RobotControlMessage(RobotControlMessage::DebugEnabledSystems(
            debug_enabled_resources.systems.clone(),
        ));

        let io = IoTaskPool::get();

        let handle = control_handle.clone();
        io.spawn(async move {
            if let Err(error) = handle.broadcast(msg).await {
                tracing::error!(?error, "Failed to send DebugEnabledSystems");
            }
        })
        .detach();
    }
}

/// A collection of system ids from registered systems. All systems are
/// associated with sending data from the robot to all connected viewers.
#[derive(Resource)]
pub struct SendRobotInitialDataSystems {
    system_ids: Vec<SystemId>,
}

impl FromWorld for SendRobotInitialDataSystems {
    fn from_world(world: &mut World) -> Self {
        let system_ids = vec![
            world.register_system(send_debug_enabled_systems),
            world.register_system(send_camera_extrinsic),
            world.register_system(send_green_chromaticity_threshold),
            world.register_system(send_game_controller_message),
            world.register_system(send_game_controller_player),
        ];

        Self { system_ids }
    }
}

pub fn send_current_state(
    control_handle: Res<ControlAppHandle>,
    time: Res<Time>,
    mut delay: Local<ControlRobotMessageDelay>,
) {
    delay.tick(time.delta());

    if !delay.finished() {
        return;
    }

    let resources = collect_resource_states(time.elapsed().as_secs().to_string());
    let msg = RobotMessage::RobotControlMessage(RobotControlMessage::Resources(resources));

    // Send/broadcast msg
    let handle = control_handle.clone();
    let io = IoTaskPool::get();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send Resource States");
        }
    })
    .detach();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> HashMap<String, String> {
    let mut resources = HashMap::new();
    resources.insert("Time".to_string(), val);
    resources
}
