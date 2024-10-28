use std::{collections::HashMap, time::Duration};

use async_std::{io::WriteExt, net::TcpStream};
use bevy::prelude::*;
use futures::{
    channel::mpsc::{self, UnboundedReceiver},
    io::WriteHalf,
    StreamExt,
};
use miette::IntoDiagnostic;
use serde::{Deserialize, Serialize};

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
    PlaceHolder,
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
        info!("Send message: {:#?}", message);
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

    info!("Stopping send messages loop")
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

    info!("Send current state");

    let state = collect_resource_states(time.elapsed().as_secs().to_string());
    sender.tx.unbounded_send(state).unwrap();

    delay.set_duration(SEND_STATE_DELAY);
}

fn collect_resource_states(val: String) -> ControlHostMessage {
    let mut resources = HashMap::new();
    resources.insert("Resource1".to_string(), val);
    resources.insert("Resource2".to_string(), "A value".to_string());
    ControlHostMessage::Resources(resources)
}
