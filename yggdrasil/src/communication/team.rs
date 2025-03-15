use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use async_std::{net::UdpSocket, prelude::StreamExt};
use bevy::{
    prelude::*,
    tasks::{block_on, IoTaskPool},
};
use futures::channel::mpsc;
use miette::IntoDiagnostic;

use crate::core::config::showtime::ShowtimeConfig;
use crate::prelude::*;
use crate::vision::referee::RefereePose;

use bifrost::broadcast::{Deadline, Inbound, Message, Outbound, Rate};
use bifrost::communication::{GameControllerMessage, GameState, Half};
use bifrost::serialization::{Decode, Encode};

/// Port range for broadcasting, the actual port is `PORT_RANGE_START + team_number`.
const PORT_RANGE_START: u16 = 10000;
/// Amount of messages remaining after the game, so we don't overshoot due to lag.
const MINIMAL_BUDGET: u16 = 5;
/// Number of seconds in a half match.
const SECS_PER_HALF: i16 = 10 * 60;

/// Plugin for communication between team members.
pub struct TeamCommunicationPlugin;

impl Plugin for TeamCommunicationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_team_communication);

        app.add_systems(Update, (ping_response, sync_budget).chain());
    }
}

fn setup_team_communication(mut commands: Commands, config: Res<ShowtimeConfig>) {
    let team_communication =
        TeamCommunication::new(config.team_number).expect("failed to create team communication.");

    commands.insert_resource(team_communication);
}

fn sync_budget(mut tc: ResMut<TeamCommunication>, message: Option<Res<GameControllerMessage>>) {
    // We can't calibrate the budget if we aren't in a game.
    if let Some(game_controller_message) = message {
        if let Some(threshold) = tc.calibrate_budget(&game_controller_message) {
            // For now, make sure to never send messages faster than can be maintained.
            tc.rate_mut().late_threshold = threshold;
            tc.rate_mut().automatic_deadline = threshold;
        }
    }

    if tc.try_send() {
        debug!("successfully sent out a new packet.")
    }

    match tc.try_receive() {
        Ok(0) => (),
        Ok(n) => debug!("received packet(s) from {} peer(s).", n),
        Err(err) => warn!(?err, "unable to receive packet"),
    }
}

fn ping_response(mut tc: ResMut<TeamCommunication>) {
    // If we have received a ping...
    let msg = tc.inbound_mut().take_map(|_, _, msg| match msg {
        TeamMessage::Ping => Some(TeamMessage::Pong),
        _ => None,
    });

    // ...send out a pong about a second later.
    if let Some((when, who, msg)) = msg {
        debug!(?who, "received ping, sending back pong");

        let at = when + Duration::from_secs(1);
        tc.outbound_mut()
            .push_by(msg, Deadline::Before(at))
            .into_diagnostic()
            .expect("failed to respond with pong message");
    }
}

#[derive(Resource)]
pub struct TeamCommunication {
    team_number: u8,
    tx: mpsc::UnboundedSender<([u8; 128], usize)>,
    rx: mpsc::UnboundedReceiver<([u8; 128], usize, SocketAddr)>,
    inbound: Inbound<SocketAddr, TeamMessage>,
    outbound: Outbound<TeamMessage>,
}

impl TeamCommunication {
    fn new(team_number: u8) -> Result<Self> {
        let io = IoTaskPool::get();
        let port = PORT_RANGE_START + u16::from(team_number);

        let socket =
            Arc::new(block_on(UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port))).into_diagnostic()?);

        socket.set_broadcast(true).into_diagnostic()?;

        let (tx, rx) = mpsc::unbounded();

        io.spawn(tx_worker(socket.clone(), (Ipv4Addr::BROADCAST, port), rx))
            .detach();

        let (tx_, rx) = mpsc::unbounded();

        io.spawn(rx_worker(socket, tx_)).detach();

        let rate = Rate {
            late_threshold: Duration::from_millis(2500),
            automatic_deadline: Duration::from_millis(5000),
            early_threshold: Duration::from_millis(7500),
        };

        Ok(Self {
            team_number,
            tx,
            rx,
            inbound: Inbound::new(),
            outbound: Outbound::new(rate),
        })
    }

    pub fn inbound_mut(&mut self) -> &mut Inbound<SocketAddr, TeamMessage> {
        &mut self.inbound
    }

    pub fn outbound_mut(&mut self) -> &mut Outbound<TeamMessage> {
        &mut self.outbound
    }

    fn rate_mut(&mut self) -> &mut Rate {
        &mut self.outbound.rate
    }

    fn try_send(&mut self) -> bool {
        if let Some(packet) = self.outbound.try_pack() {
            let mut buf = [0; 128];
            let mut len = 0;

            for (buf_i, packet_i) in buf.iter_mut().zip(packet) {
                *buf_i = packet_i;
                len += 1;
            }

            self.tx.unbounded_send((buf, len)).unwrap();

            true
        } else {
            false
        }
    }

    fn try_receive(&mut self) -> Result<usize> {
        let mut received = 0;

        while let Some((buf, len, addr)) = self.rx.try_next().into_diagnostic()? {
            self.inbound.unpack(&buf[..len], addr).into_diagnostic()?;
            received += 1;
        }

        Ok(received)
    }

    fn calibrate_budget(&self, message: &GameControllerMessage) -> Option<Duration> {
        let GameControllerMessage {
            state: GameState::Playing,
            players_per_team,
            mut secs_remaining,
            first_half,
            ..
        } = message
        else {
            return None;
        };

        if *first_half == Half::First {
            secs_remaining += SECS_PER_HALF;
        }

        let team_info = message.team(self.team_number)?;
        let messages = f32::from(team_info.message_budget.saturating_sub(MINIMAL_BUDGET));
        let messages_per_player = messages / f32::from(*players_per_team);
        let secs_per_message = f32::from(secs_remaining.max(0)) / messages_per_player;

        Some(Duration::from_secs_f32(secs_per_message))
    }
}

async fn tx_worker(
    socket: Arc<UdpSocket>,
    addr: (Ipv4Addr, u16),
    mut rx: mpsc::UnboundedReceiver<([u8; 128], usize)>,
) {
    while let Some((buf, len)) = rx.next().await {
        if socket.send_to(&buf[..len], addr).await.is_err() {
            tracing::warn!("unable to send packet");
        }
    }
}
async fn rx_worker(
    socket: Arc<UdpSocket>,
    tx: mpsc::UnboundedSender<([u8; 128], usize, SocketAddr)>,
) {
    loop {
        let mut buf = [0; 128];

        let Ok((len, addr)) = socket.recv_from(&mut buf).await else {
            tracing::warn!("unable to receive packet");
            continue;
        };

        tx.unbounded_send((buf, len, addr)).ok();
    }
}

#[derive(Debug, Encode, Decode)]
#[non_exhaustive]
pub enum TeamMessage {
    Ping,
    Pong,
    DetectedWhistle,
    RecognizedRefereePose(RefereePose),
}

impl Message for TeamMessage {
    const MAX_PACKET_SIZE: usize = 128;
    const EXPECTED_SIZE: usize = 1;
    const DEAD_SPACE: usize = 16;

    fn try_merge(&mut self, old: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(old)
    }
}
