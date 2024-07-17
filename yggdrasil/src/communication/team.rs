use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use miette::IntoDiagnostic;

use crate::core::config::showtime::ShowtimeConfig;
use crate::prelude::*;

use bifrost::broadcast::*;
use bifrost::communication::{GameControllerMessage, GameState, Half};
use bifrost::serialization::{Decode, Encode};

/// Port range for broadcasting, the actual port is `PORT_RANGE_START + team_number`.
const PORT_RANGE_START: u16 = 10000;
/// Amount of messages remaining after the game, so we don't overshoot due to lag.
const MINIMAL_BUDGET: u16 = 5;
/// Number of seconds in a half match.
const SECS_PER_HALF: i16 = 10 * 60;

pub struct TeamCommunicationModule;

impl Module for TeamCommunicationModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(sync)
            .add_system(ping_response)
            .add_startup_system(startup)
    }
}

#[startup_system]
fn startup(storage: &mut Storage, config: &ShowtimeConfig) -> Result<()> {
    let tc = TeamCommunication::new(config.team_number);
    storage.add_resource(Resource::new(tc?))
}

#[system]
fn sync(tc: &mut TeamCommunication, message: &Option<GameControllerMessage>) -> Result<()> {
    // We can't calibrate the budget if we aren't in a game.
    let Some(game_controller_message) = message else {
        return Ok(());
    };

    if let Some(threshold) = tc.calibrate_budget(game_controller_message) {
        // For now, make sure to never send messages faster than can be maintained.
        tc.rate_mut().late_threshold = threshold;
        tc.rate_mut().automatic_deadline = threshold;

        if tc.try_send()? {
            tracing::info!("successfully sent out a new packet.");
        }

        let received = tc.try_receive()?;
        if received > 0 {
            tracing::info!("received packet(s) from {} peer(s).", received);
        }
    }

    Ok(())
}

#[system]
fn ping_response(tc: &mut TeamCommunication) -> Result<()> {
    // If we have received a ping...
    let msg = tc.inbound_mut().take_map(|_, _, msg| match msg {
        TeamMessage::Ping => Some(TeamMessage::Pong),
        _ => None,
    });

    // ...send out a pong about a second later.
    if let Some((when, who, msg)) = msg {
        tracing::info!("{:?} said ping, i say pong", who);

        let at = when + Duration::from_secs(1);
        tc.outbound_mut()
            .push_by(msg, Deadline::Before(at))
            .into_diagnostic()?;
    }

    Ok(())
}

pub struct TeamCommunication {
    port: u16,
    team_number: u8,
    socket: UdpSocket,
    inbound: Inbound<SocketAddr, TeamMessage>,
    outbound: Outbound<TeamMessage>,
}

impl TeamCommunication {
    fn new(team_number: u8) -> Result<Self> {
        let port = PORT_RANGE_START + team_number as u16;

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
        } = message
        else {
            return None;
        };

        if *first_half == Half::First {
            secs_remaining += SECS_PER_HALF;
        }

        let team_info = message.team(self.team_number)?;
        let messages = team_info.message_budget.saturating_sub(MINIMAL_BUDGET) as f32;
        let messages_per_player = messages / *players_per_team as f32;
        let secs_per_message = secs_remaining.max(0) as f32 / messages_per_player;

        Some(Duration::from_secs_f32(secs_per_message))
    }
}

#[derive(Debug, Encode, Decode)]
#[non_exhaustive]
pub enum TeamMessage {
    Ping,
    Pong,
}

impl Message for TeamMessage {
    const MAX_PACKET_SIZE: usize = 128;
    const EXPECTED_SIZE: usize = 1;
    const DEAD_SPACE: usize = 16;
}
