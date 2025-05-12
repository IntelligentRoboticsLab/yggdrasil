use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use bevy::prelude::{App, *};
use miette::IntoDiagnostic;
use tracing::{debug, warn};

use crate::core::config::showtime::ShowtimeConfig;
use crate::prelude::Result;
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

    match tc.try_send() {
        Ok(true) => debug!("successfully sent out a new packet."),
        Ok(false) => (),
        Err(err) => warn!(?err, "unable to send packet"),
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
    port: u16,
    team_number: u8,
    socket: UdpSocket,
    inbound: Inbound<SocketAddr, TeamMessage>,
    outbound: Outbound<TeamMessage>,
}

impl TeamCommunication {
    fn new(team_number: u8) -> Result<Self> {
        let port = PORT_RANGE_START + u16::from(team_number);

        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, port)).into_diagnostic()?;
        socket.set_nonblocking(true).into_diagnostic()?;
        socket.set_broadcast(true).into_diagnostic()?;

        let rate = Rate {
            late_threshold: Duration::from_millis(2500),
            automatic_deadline: Duration::from_millis(5000),
            early_threshold: Duration::from_millis(7500),
        };

        Ok(Self {
            port,
            team_number,
            socket,
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

    fn try_send(&mut self) -> Result<bool> {
        if let Some(packet) = self.outbound.try_pack() {
            match self
                .socket
                .send_to(&packet, (Ipv4Addr::BROADCAST, self.port))
            {
                Ok(_) => Ok(true),
                Err(e) if e.kind() == ErrorKind::WouldBlock => Ok(false),
                Err(e) => Err(e).into_diagnostic(),
            }
        } else {
            Ok(false)
        }
    }

    fn try_receive(&mut self) -> Result<usize> {
        let mut received = 0;
        let mut buf = [0; 128];

        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    received += 1;
                    self.inbound.unpack(&buf[..len], addr).into_diagnostic()?;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => return Err(e).into_diagnostic(),
            }
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
        } = *message
        else {
            return None;
        };

        if first_half == Half::First {
            secs_remaining += SECS_PER_HALF;
        }

        let team_info = message.team(self.team_number)?;
        let messages = f32::from(team_info.message_budget.saturating_sub(MINIMAL_BUDGET));
        let messages_per_player = messages / f32::from(players_per_team);
        let secs_per_message = f32::from(secs_remaining.max(0)) / messages_per_player;

        Some(Duration::from_secs_f32(secs_per_message))
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
    const DEAD_SPACE: usize = 64;

    fn try_merge(&mut self, old: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(old)
    }
}
