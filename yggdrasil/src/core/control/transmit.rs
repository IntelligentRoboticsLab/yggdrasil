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
pub struct ControlHostMessage {
    pub resources: HashMap<String, String>,
}

#[derive(Resource)]
pub struct ControlSender {
    pub tx: mpsc::UnboundedSender<ControlHostMessage>,
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
        stream
            .write_all(&serialized_msg)
            .await
            .expect("Failed writing the control message to the stream");
    }

    info!("Stopping send messages loop")
}

pub fn send_current_state(
    sender: Res<ControlSender>,
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
    ControlHostMessage { resources }
}
