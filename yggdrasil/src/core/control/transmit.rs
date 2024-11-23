use std::{collections::HashMap, sync::Arc, time::Duration};

use async_std::{io::WriteExt, net::TcpStream};
use bevy::{ecs::system::SystemId, prelude::*, tasks::{futures_lite::stream::block_on, IoTaskPool}};
use control::connection::{app::ControlAppHandle, protocol::{RobotMessage, ViewerMessage}};
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    io::WriteHalf,
    StreamExt,
};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};
use bifrost::serialization::Encode;

use super::DebugEnabledSystems;

const SEND_STATE_DELAY: Duration = Duration::from_millis(2_000);

#[derive(Deref, DerefMut)]
pub struct ControlRobotMessageDelay(Timer);

impl Default for ControlRobotMessageDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

#[derive(Resource, Serialize, Deserialize, Debug)]
pub enum ControlRobotMessage {
    CloseStream,
    Resources(HashMap<String, String>),
    DebugEnabledSystems(DebugEnabledSystems),
}

#[derive(Resource)]
pub struct ControlSender<T> {
    pub tx: mpsc::UnboundedSender<T>,
}

pub async fn send_messages(
    mut stream: WriteHalf<TcpStream>,
    mut receiver: UnboundedReceiver<ControlRobotMessage>,
) {
    while let Some(message) = receiver.next().await {
        tracing::debug!("Send message: {:#?}", message);
        let serialized_msg = bincode::serialize(&message)
            .into_diagnostic()
            .expect("Was not able to serialize a ControlRobotMessage");

        let msg_size = serialized_msg.len();
        let serialized_msg_size = bincode::serialize(&msg_size).into_diagnostic().unwrap();
        // Send the size of the message first
        stream
            .write(&serialized_msg_size)
            .await
            .expect("Failed writing the control message size to the stream");
        // Send the actual robot message
        stream
            .write_all(&serialized_msg)
            .await
            .expect("Failed writing the control message to the stream");
    }

    tracing::warn!("Stopping send messages loop");
}

pub fn send_current_state(
    control_handle: Res<ControlAppHandle<RobotMessage, ViewerMessage>>,
    time: Res<Time>,
    mut delay: Local<ControlRobotMessageDelay>,
) {
    delay.tick(time.delta());

    if !delay.finished() {
        return;
    }

    let resources = collect_resource_states(time.elapsed().as_secs().to_string());
    let msg = RobotMessage::Resources(resources);
    tracing::info!("Send message of {} bytes", msg.encode_len());

    // Send/broadcast msg
    let handle = control_handle.clone();
    let io = IoTaskPool::get();
    io.spawn(async move {
        tracing::info!("Prepare send state msg");
        handle.broadcast(msg).await;
        tracing::info!("Finish send state msg");
    }).detach();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> HashMap<String, String> {
    let mut resources = HashMap::new();
    resources.insert("Time".to_string(), val);
    resources
}

#[derive(Resource)]
pub struct TransmitDebugEnabledSystems {
    system_id: SystemId,
}

impl TransmitDebugEnabledSystems {
    #[must_use]
    pub fn system_id(&self) -> SystemId {
        self.system_id
    }
}

impl FromWorld for TransmitDebugEnabledSystems {
    fn from_world(world: &mut World) -> Self {
        let system_id = world.register_system(transmit_debug_enabled_resources);
        TransmitDebugEnabledSystems { system_id }
    }
}

fn transmit_debug_enabled_resources(
    debug_enabled_resources: Res<DebugEnabledSystems>,
    robot_handle: Res<ControlAppHandle<RobotMessage, ViewerMessage>>,
) {
    let msg = RobotMessage::DebugEnabledSystems(debug_enabled_resources.clone());
    let io = IoTaskPool::get();

    let handle = robot_handle.clone();
    io.spawn(async move {
        tracing::info!("Running the task to broadcast the robot message");
        handle.broadcast(msg).await;
    }).detach();
}

pub fn temp_system(type_registry: Res<AppTypeRegistry>) {
    let registry = type_registry.read();

    let resources: Vec<_> = registry
        .iter()
        .filter(|registration| registration.data::<ReflectResource>().is_some())
        .map(|registration| registration.type_info().type_path())
        .collect();

    tracing::info!("Registry: {:#?}", resources);
}
