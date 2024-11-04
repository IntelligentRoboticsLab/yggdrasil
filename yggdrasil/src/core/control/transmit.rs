use std::{collections::HashMap, time::Duration};

use async_std::{io::WriteExt, net::TcpStream};
use bevy::{ecs::system::SystemId, prelude::*};
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    io::WriteHalf,
    StreamExt,
};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

use super::DebugEnabledSystems;

const SEND_STATE_DELAY: Duration = Duration::from_millis(2_000);

#[derive(Deref, DerefMut)]
pub struct ControlHostMessageDelay(Timer);

impl Default for ControlHostMessageDelay {
    fn default() -> Self {
        Self(Timer::new(Duration::ZERO, TimerMode::Repeating))
    }
}

#[derive(Resource, Serialize, Deserialize, Debug)]
pub enum ControlHostMessage {
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
    mut receiver: UnboundedReceiver<ControlHostMessage>,
) {
    while let Some(message) = receiver.next().await {
        tracing::debug!("Send message: {:#?}", message);
        let serialized_msg = bincode::serialize(&message)
            .into_diagnostic()
            .expect("Was not able to serialize a ControlHostMessage");

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
    sender: Res<ControlSender<ControlHostMessage>>,
    time: Res<Time>,
    mut delay: Local<ControlHostMessageDelay>,
) {
    delay.tick(time.delta());

    if !delay.finished() {
        return;
    }

    let state = collect_resource_states(time.elapsed().as_secs().to_string());
    sender.tx.unbounded_send(state).unwrap();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> ControlHostMessage {
    let mut resources = HashMap::new();
    resources.insert("Time".to_string(), val);
    ControlHostMessage::Resources(resources)
}

#[derive(Resource)]
pub struct TransmitDebugEnabledSystems {
    system_id: SystemId,
}

impl TransmitDebugEnabledSystems {
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
    sender: Res<ControlSender<ControlHostMessage>>,
) {
    let message = ControlHostMessage::DebugEnabledSystems(debug_enabled_resources.clone());
    sender.tx.unbounded_send(message).unwrap();
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
