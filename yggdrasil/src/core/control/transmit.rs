use std::{collections::HashMap, time::Duration};

use super::{events::DebugEnabledSystemUpdated, ViewerConnected};
use bevy::{prelude::*, tasks::IoTaskPool};
use re_control_comms::{
    app::ControlAppHandle, debug_system::DebugEnabledSystems, protocol::RobotMessage,
};

const SEND_STATE_DELAY: Duration = Duration::from_millis(2_000);

#[derive(Deref, DerefMut)]
pub struct ControlRobotMessageDelay(Timer);

impl Default for ControlRobotMessageDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

// Sends the current state of `DebugEnabledSystems` to the client that
// connected.
pub fn debug_systems_on_new_connection(
    mut ev_viewer_connected: EventReader<ViewerConnected>,
    debug_enabled_resources: Res<DebugEnabledSystems>,
    control_handle: Res<ControlAppHandle>,
) {
    for ev in ev_viewer_connected.read() {
        let viewer_id = ev.0;

        let msg = RobotMessage::DebugEnabledSystems(debug_enabled_resources.systems.clone());

        let io = IoTaskPool::get();

        let handle = control_handle.clone();
        io.spawn(async move {
            handle.send(msg, viewer_id).await;
        })
        .detach();
    }
}

// This system sends the current `DebugEnabledSystems` to all connected
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
            handle.broadcast(msg).await;
        })
        .detach();
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
        handle.broadcast(msg).await;
    })
    .detach();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> HashMap<String, String> {
    let mut resources = HashMap::new();
    resources.insert("Time".to_string(), val);
    resources
}
