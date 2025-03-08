use std::{collections::HashMap, time::Duration};

use bevy::{ecs::system::SystemId, prelude::*, tasks::IoTaskPool};
use heimdall::CameraPosition;
use re_control_comms::{
    app::ControlAppHandle, debug_system::DebugEnabledSystems, protocol::RobotMessage,
};

use crate::vision::{camera::CameraConfig, scan_lines::ScanLinesConfig};

use super::{DebugEnabledSystemUpdated, ViewerConnected};

const SEND_STATE_DELAY: Duration = Duration::from_millis(2_000);

#[derive(Deref, DerefMut)]
pub struct ControlRobotMessageDelay(Timer);

impl Default for ControlRobotMessageDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

pub fn send_on_connection(
    mut commands: Commands,
    mut viewer_events: EventReader<ViewerConnected>,
    send_debug_enabled_systems: Res<SendDebugEnabledSystems>,
    send_camera_extrinsic: Res<SendCameraExtrinsic>,
    send_green_chromaticity_threshold: Res<SendGreenChromaticityThreshold>,
) {
    if !viewer_events.is_empty() {
        viewer_events.clear();

        commands.run_system(send_debug_enabled_systems.0);
        commands.run_system(send_camera_extrinsic.0);
        commands.run_system(send_green_chromaticity_threshold.0);
    }
}

// Sends the current state of `DebugEnabledSystems` to the client that
// connected.
pub fn send_debug_enabled_systems(
    debug_enabled_resources: Res<DebugEnabledSystems>,
    control_handle: Res<ControlAppHandle>,
) {
    let msg = RobotMessage::DebugEnabledSystems(debug_enabled_resources.systems.clone());

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

    let top_msg = RobotMessage::CameraExtrinsic {
        camera_position: CameraPosition::Top,
        extrinsic_rotation: top_extrinsic,
    };

    let bot_msg = RobotMessage::CameraExtrinsic {
        camera_position: CameraPosition::Bottom,
        extrinsic_rotation: bottom_extrinsic,
    };

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
    let msg = RobotMessage::FieldColor {
        config: scan_lines_config.as_ref().into(),
    };

    let io = IoTaskPool::get();

    let handle = control_handle.clone();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send green chromaticity threshold");
        }
    })
    .detach();
}

// This system sends the current [`DebugEnabledSystems`] to all connected
// clients.
// When an individual client updates a debug enabled system, the state of
// other connected clients should also be updated.
pub fn update_debug_systems_for_clients(
    debug_enabled_resources: Res<DebugEnabledSystems>,
    control_handle: Res<ControlAppHandle>,
    mut ev_debug_enabled_system_updated: EventReader<DebugEnabledSystemUpdated>,
) {
    for _ev in ev_debug_enabled_system_updated.read() {
        let msg = RobotMessage::DebugEnabledSystems(debug_enabled_resources.systems.clone());

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

#[derive(Resource)]
pub struct SendDebugEnabledSystems(SystemId);

impl FromWorld for SendDebugEnabledSystems {
    fn from_world(world: &mut World) -> Self {
        let system_id = world.register_system(send_debug_enabled_systems);
        Self(system_id)
    }
}

#[derive(Resource)]
pub struct SendCameraExtrinsic(SystemId);

impl FromWorld for SendCameraExtrinsic {
    fn from_world(world: &mut World) -> Self {
        let system_id = world.register_system(send_camera_extrinsic);
        Self(system_id)
    }
}

#[derive(Resource)]
pub struct SendGreenChromaticityThreshold(SystemId);

impl FromWorld for SendGreenChromaticityThreshold {
    fn from_world(world: &mut World) -> Self {
        let system_id = world.register_system(send_green_chromaticity_threshold);
        Self(system_id)
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
    let msg = RobotMessage::Resources(resources);

    // Send/broadcast msg
    let handle = control_handle.clone();
    let io = IoTaskPool::get();
    io.spawn(async move {
        if let Err(error) = handle.broadcast(msg).await {
            tracing::error!(?error, "Failed to send Resource States");
        };
    })
    .detach();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> HashMap<String, String> {
    let mut resources = HashMap::new();
    resources.insert("Time".to_string(), val);
    resources
}
